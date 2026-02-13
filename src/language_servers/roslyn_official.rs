use zed_extension_api::{self as zed, settings::LspSettings, LanguageServerId, Result};

pub struct RoslynOfficial {}

impl RoslynOfficial {
    pub const LANGUAGE_SERVER_ID: &'static str = "roslyn-official";

    pub fn new() -> Self {
        RoslynOfficial {}
    }

    pub fn language_server_cmd(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let Some(lsp_settings) = LspSettings::for_worktree(Self::LANGUAGE_SERVER_ID, worktree).ok()
        else {
            return Err(format!("Unable to load settings"));
        };

        let binary_args = lsp_settings
            .binary
            .as_ref()
            .and_then(|binary_settings| binary_settings.arguments.clone());

        // Build arguments list
        let mut args = vec!["--stdio".to_string(), "--autoLoadProjects".to_string()];

        let no_settings_error_message = format!(
            r#"
            No settings configured for roslyn-official.
            Please specify the razor_source_repository_root in your settings.json:
            "lsp": {{
              "roslyn-official": {{
                "settings": {{
                  "razor_source_repository_root": "/some/place/nice/razor"
                }}
              }}
            }},
            "#
        );

        let lsp_user_settings = match lsp_settings.settings {
            Some(settings) => settings,
            None => {
                return Err(no_settings_error_message);
            }
        };

        let razor_root = lsp_user_settings["razor_source_repository_root"]
            .as_str()
            .map(|s| s.to_string());

        let razor_root_unwrapped = &match razor_root {
            None => return Err(no_settings_error_message),
            Some(x) => x,
        };

        let (sdk_path, sdk_version) = Self::find_dotnet_sdk_path(worktree)?;

        let env_vars = worktree.shell_env();

        let directory_exists = zed_extension_api::Command::new("test")
            .arg("-d")
            .arg(razor_root_unwrapped)
            .output()?;

        if directory_exists.status.unwrap() == 0 {
            zed_extension_api::set_language_server_installation_status(
                _language_server_id,
                &zed::LanguageServerInstallationStatus::CheckingForUpdate,
            );
            // in this case we can reset the git repository and pull the latest changes
            let razor_root_reset = zed_extension_api::process::Command::new("git")
                .arg("-C")
                .arg(razor_root_unwrapped)
                .arg("reset")
                .arg("--hard")
                .output()?;

            if razor_root_reset.status.is_none() || razor_root_reset.status.unwrap() != 0 {
                return Err(format!(
                    "Unable to reset razor git repository. Git installed?"
                ));
            }

            zed_extension_api::set_language_server_installation_status(
                _language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );
            let razor_root_git_pull = zed_extension_api::process::Command::new("git")
                .arg("-C")
                .arg(razor_root_unwrapped)
                .arg("pull")
                .output()?;

            if razor_root_git_pull.status.is_none() || razor_root_git_pull.status.unwrap() != 0 {
                println!("Unable to pull latest changes in razor repository. Offline?");
            }
        } else {
            // in this case we need to clone the repository
            zed_extension_api::set_language_server_installation_status(
                _language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            let razor_root_clone = zed_extension_api::process::Command::new("git")
                .arg("clone")
                .arg("https://github.com/dotnet/razor")
                .arg(razor_root_unwrapped)
                .output()?;

            if razor_root_clone.status.is_none() || razor_root_clone.status.unwrap() != 0 {
                return Err(format!("Unable to clone razor git repository. For this initial setup step, an internet connection is required."));
            }
        }

        let dotnet_build = zed_extension_api::process::Command::new("dotnet")
            .arg("build")
            .arg(format!(
                "{}/src/Razor/src/Microsoft.VisualStudioCode.RazorExtension/Microsoft.VisualStudioCode.RazorExtension.csproj",
                razor_root_unwrapped
            ))
            .arg("--configuration").arg("Release")
            .envs(env_vars)
            .output()?;

        if dotnet_build.status.is_none() || dotnet_build.status.unwrap() != 0 {
            return Err(format!("Unable to build razor extension"));
        }

        // if someone knows a better way to get this dll, i'm all ears
        let sdk_version_short = format!(
            "net{}",
            sdk_version.split('.').take(2).collect::<Vec<_>>().join(".")
        );
        let razor_vscode_extension_path = format!("{}/artifacts/bin/Microsoft.VisualStudioCode.RazorExtension/Release/{}/Microsoft.VisualStudioCode.RazorExtension.dll", razor_root_unwrapped, sdk_version_short);

        args.push("--extension".to_string());
        args.push(razor_vscode_extension_path);

        let razor_dll = Self::find_razor_compiler_dll(&sdk_path);
        let razor_targets = Self::find_razor_design_time_targets(&sdk_path);

        // Add Razor source generator argument
        args.push("--razorSourceGenerator".to_string());
        args.push(razor_dll);

        // Add Razor design time path argument
        args.push("--razorDesignTimePath".to_string());
        args.push(razor_targets);

        // Use custom args if provided, otherwise use our built args
        let final_args = binary_args.unwrap_or(args);

        // Check if user provided a custom path
        if let Some(path) = lsp_settings
            .binary
            .and_then(|binary_settings| binary_settings.path)
        {
            return Ok(zed::Command {
                command: path,
                args: final_args,
                env: Default::default(),
            });
        }

        // Try to find roslyn-language-server in PATH
        if let Some(path) = worktree.which("roslyn-language-server") {
            // try to update
            zed_extension_api::set_language_server_installation_status(
                _language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );
            zed_extension_api::process::Command::new("dotnet")
                .arg("tool")
                .arg("update")
                .arg("roslyn-language-server")
                .arg("--global")
                .arg("--prerelease")
                .output()?;

            return Ok(zed::Command {
                command: path,
                args: final_args,
                env: Default::default(),
            });
        } else {
            // download the tool
            zed::set_language_server_installation_status(
                _language_server_id,
                &zed_extension_api::LanguageServerInstallationStatus::Downloading,
            );
            zed_extension_api::process::Command::new("dotnet")
                .arg("tool")
                .arg("install")
                .arg("--global")
                .arg("roslyn-language-server")
                .arg("--prerelease")
                .output()?;

            // check again
            if let Some(path) = worktree.which("roslyn-language-server") {
                return Ok(zed::Command {
                    command: path,
                    args: final_args,
                    env: Default::default(),
                });
            }
        }

        Err(format!(
            "roslyn-language-server not found or unable to install. Please try installing it manually using: 'dotnet tool install --global roslyn-language-server --prerelease'"
        ))
    }

    fn find_dotnet_sdk_path(worktree: &zed::Worktree) -> Result<(String, String), String> {
        // Run `dotnet --list-sdks` to get all installed SDKs
        let env_vars = worktree.shell_env();
        let output = zed_extension_api::process::Command::new("dotnet")
            .arg("--list-sdks")
            .envs(env_vars)
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse the output to get the latest SDK version
        // Format: "10.0.100 [/home/user/.dotnet/sdk]"
        let last_line = stdout.lines().last();
        if last_line.is_none() {
            return Err(format!("Unable to parse dotnet sdk info output {}", stdout));
        }

        let parts: Vec<&str> = last_line.unwrap().split_whitespace().collect();
        let sdk_version = if parts.len() >= 2 {
            let version = parts[0];
            let path = parts[1].trim_matches(|c| c == '[' || c == ']');
            Some((version.to_string(), path.to_string()))
        } else {
            None
        };
        if sdk_version.is_none() {
            return Err(format!("Unable to parse sdk info otput {}", stdout));
        }

        return Ok(sdk_version
            .map(|(version, base_path)| (format!("{}/{}", base_path, version), version))
            .unwrap());
    }

    /// Find the Razor Compiler DLL in the given SDK path
    fn find_razor_compiler_dll(sdk_path: &str) -> String {
        let dll_path = format!(
            "{}/Sdks/Microsoft.NET.Sdk.Razor/source-generators/Microsoft.CodeAnalysis.Razor.Compiler.dll",
            sdk_path
        );

        return dll_path;
    }

    /// Find the Razor Design Time Targets file in the given SDK path
    fn find_razor_design_time_targets(sdk_path: &str) -> String {
        return format!(
            "{}/Sdks/Microsoft.NET.Sdk.Razor/targets/Microsoft.NET.Sdk.Razor.DesignTime.targets",
            sdk_path
        );
    }

    pub fn configuration_options(
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        let settings = LspSettings::for_worktree(Self::LANGUAGE_SERVER_ID, worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.settings);

        Ok(settings.map(Self::transform_settings_for_roslyn))
    }

    fn transform_settings_for_roslyn(settings: zed::serde_json::Value) -> zed::serde_json::Value {
        let mut roslyn_config = zed::serde_json::json!({
            // Enable Razor cohosting for proper Razor/Blazor support
            "razor|language_server.cohosting_enabled": true,
            // These code lenses show up as "Unknown Command" in Zed and don't do anything when clicked. Disable them by default.
            "csharp|code_lens.dotnet_enable_references_code_lens": false,
            "csharp|code_lens.dotnet_enable_tests_code_lens": false,
            // Disable code lenses for Razor files to prevent errors
            "razor|code_lens.dotnet_enable_references_code_lens": false,
            "razor|code_lens.dotnet_enable_tests_code_lens": false,
            // Enable inlay hints in the language server by default.
            // This way, enabling inlay hints in Zed will cause inlay hints to show up in C# without extra configuration.
            "csharp|inlay_hints.dotnet_enable_inlay_hints_for_parameters": true,
            "csharp|inlay_hints.dotnet_enable_inlay_hints_for_literal_parameters": true,
            "csharp|inlay_hints.dotnet_enable_inlay_hints_for_indexer_parameters": true,
            "csharp|inlay_hints.dotnet_enable_inlay_hints_for_object_creation_parameters": true,
            "csharp|inlay_hints.dotnet_enable_inlay_hints_for_other_parameters": true,
            "csharp|inlay_hints.csharp_enable_inlay_hints_for_types": true,
            "csharp|inlay_hints.csharp_enable_inlay_hints_for_implicit_variable_types": true,
            "csharp|inlay_hints.csharp_enable_inlay_hints_for_lambda_parameter_types": true,
            "csharp|inlay_hints.csharp_enable_inlay_hints_for_implicit_object_creation": true,
            "csharp|inlay_hints.csharp_enable_inlay_hints_for_collection_expressions": true,
        });

        let config_map = roslyn_config.as_object_mut().unwrap();
        if let zed::serde_json::Value::Object(settings_map) = settings {
            for (key, value) in settings_map {
                if key.contains('|') {
                    // This is already in the language|category format
                    if let zed::serde_json::Value::Object(nested_settings) = value {
                        for (nested_key, nested_value) in nested_settings {
                            // The key already contains the proper format, just add the setting
                            config_map.insert(format!("{key}.{nested_key}"), nested_value);
                        }
                    }
                }
                // Handle direct roslyn-format settings (fallback for any other format)
                else if key.contains('.') {
                    config_map.insert(key.clone(), value.clone());
                }
            }
        }

        roslyn_config
    }
}
