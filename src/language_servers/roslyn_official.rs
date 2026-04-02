use std::{env, fs, path::PathBuf};
use zed_extension_api::{self as zed, settings::LspSettings, LanguageServerId, Result};

pub struct RoslynOfficial {}

type EnvVars = Vec<(String, String)>;

fn with_optional_env(
    command: zed_extension_api::process::Command,
    dotnet_env: Option<&EnvVars>,
) -> zed_extension_api::process::Command {
    match dotnet_env {
        Some(env_vars) => command.envs(env_vars.clone()),
        None => command,
    }
}

fn new_dotnet_command(
    dotnet_command: &str,
    dotnet_env: Option<&EnvVars>,
) -> zed_extension_api::process::Command {
    let command = with_optional_env(
        zed_extension_api::process::Command::new(dotnet_command),
        dotnet_env,
    );

    command.envs(dotnet_execution_env(dotnet_command, dotnet_env))
}

fn dotnet_execution_env(dotnet_command: &str, dotnet_env: Option<&EnvVars>) -> EnvVars {
    let dotnet_root = std::path::Path::new(dotnet_command)
        .parent()
        .map(|path| path.to_string_lossy().to_string());
    let Some(dotnet_root) = dotnet_root else {
        return vec![];
    };

    let mut env_vars = vec![
        ("DOTNET_ROOT".to_string(), dotnet_root.clone()),
        ("DOTNET_HOST_PATH".to_string(), dotnet_command.to_string()),
    ];

    let current_path = dotnet_env
        .and_then(|env_vars| {
            env_vars
                .iter()
                .find(|(key, _)| key == "PATH")
                .map(|(_, value)| value.clone())
        })
        .or_else(|| env::var("PATH").ok());

    if let Some(path) = current_path {
        env_vars.push(("PATH".to_string(), format!("{}:{}", dotnet_root, path)));
    }

    env_vars
}

impl RoslynOfficial {
    pub const LANGUAGE_SERVER_ID: &'static str = "roslyn-official";

    pub fn new() -> Self {
        RoslynOfficial {}
    }

    pub fn language_server_cmd(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let Some(lsp_settings) = LspSettings::for_worktree(Self::LANGUAGE_SERVER_ID, worktree).ok()
        else {
            return Err(format!("Unable to load settings"));
        };
        let use_worktree_dotnet_env = Self::use_worktree_dotnet_env(&lsp_settings);
        let dotnet_env = if use_worktree_dotnet_env {
            Some(worktree.shell_env())
        } else {
            None
        };
        let dotnet_command = Self::resolve_dotnet_command(dotnet_env.as_ref());

        // Build arguments list
        let base_args = vec!["--stdio".to_string(), "--autoLoadProjects".to_string()];

        let razor_args = Self::install_or_update_razor(
            lsp_settings,
            language_server_id,
            &dotnet_command,
            dotnet_env.as_ref(),
        )?;

        let final_args = match razor_args {
            Some(x) => base_args.into_iter().chain(x).collect(),
            None => base_args,
        };

        // Force the selected dotnet host/root into every Roslyn child process.
        let env = dotnet_execution_env(&dotnet_command, dotnet_env.as_ref());

        // Try to find roslyn-language-server in PATH
        if let Some(path) = worktree.which("roslyn-language-server") {
            zed_extension_api::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::CheckingForUpdate,
            );

            if let Err(error) = update_roslyn_server(&dotnet_command, dotnet_env.as_ref()) {
                println!("Unable to update roslyn-language-server: {}", error);
            }

            zed_extension_api::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::None,
            );

            return Ok(zed::Command {
                command: path,
                args: final_args,
                env: env,
            });
        } else {
            zed_extension_api::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            download_roslyn_server(&dotnet_command, dotnet_env.as_ref())?;

            // check again
            if let Some(path) = worktree.which("roslyn-language-server") {
                zed_extension_api::set_language_server_installation_status(
                    language_server_id,
                    &zed::LanguageServerInstallationStatus::None,
                );

                return Ok(zed::Command {
                    command: path,
                    args: final_args,
                    env: env,
                });
            }
        }

        Err(format!(
            "roslyn-language-server not found or unable to install. Please try installing it manually using: 'dotnet tool install --global roslyn-language-server --prerelease'"
        ))
    }

    fn use_worktree_dotnet_env(lsp_settings: &LspSettings) -> bool {
        lsp_settings
            .settings
            .as_ref()
            .and_then(|settings| settings.get("use_worktree_dotnet_env"))
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
    }

    fn resolve_dotnet_command(dotnet_env: Option<&EnvVars>) -> String {
        if dotnet_env.is_some() {
            return Self::find_active_dotnet_command(dotnet_env)
                .unwrap_or_else(|| "dotnet".to_string());
        }

        Self::find_latest_side_by_side_dotnet_command(dotnet_env)
            .or_else(|| Self::find_active_dotnet_command(dotnet_env))
            .unwrap_or_else(|| "dotnet".to_string())
    }

    fn find_active_dotnet_command(dotnet_env: Option<&EnvVars>) -> Option<String> {
        let mut command = with_optional_env(
            zed_extension_api::process::Command::new("which").arg("dotnet"),
            dotnet_env,
        );
        let output = command.output().ok()?;
        if output.status != Some(0) {
            return None;
        }

        let dotnet_command = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if dotnet_command.is_empty() {
            return None;
        }

        Some(dotnet_command)
    }

    fn find_latest_side_by_side_dotnet_command(dotnet_env: Option<&EnvVars>) -> Option<String> {
        let active_dotnet_command = Self::find_active_dotnet_command(dotnet_env)?;
        let canonical_dotnet_path = fs::canonicalize(&active_dotnet_command)
            .unwrap_or_else(|_| PathBuf::from(&active_dotnet_command));
        let active_dotnet_root = canonical_dotnet_path.parent()?;
        let installs_root = active_dotnet_root.parent()?;
        let active_version = Self::parse_dotnet_version(active_dotnet_root.file_name()?.to_str()?)?;

        let mut best_match = (active_version, canonical_dotnet_path.clone());

        for entry in fs::read_dir(installs_root).ok()? {
            let entry = entry.ok()?;
            let entry_path = entry.path();

            if !entry_path.is_dir() {
                continue;
            }

            let Some(version) = entry_path
                .file_name()
                .and_then(|name| name.to_str())
                .and_then(Self::parse_dotnet_version)
            else {
                continue;
            };

            let candidate_dotnet = entry_path.join("dotnet");
            if !candidate_dotnet.is_file() || version <= best_match.0 {
                continue;
            }

            best_match = (version, candidate_dotnet);
        }

        Some(best_match.1.to_string_lossy().to_string())
    }

    fn parse_dotnet_version(version: &str) -> Option<Vec<u32>> {
        let parsed = version
            .split('.')
            .map(|part| part.parse::<u32>().ok())
            .collect::<Option<Vec<_>>>()?;

        if parsed.is_empty() {
            return None;
        }

        Some(parsed)
    }

    fn find_dotnet_sdk_path(
        dotnet_command: &str,
        dotnet_env: Option<&EnvVars>,
    ) -> Result<(String, String), String> {
        let mut sdk_list_command =
            new_dotnet_command(dotnet_command, dotnet_env).arg("--list-sdks");
        let sdks_output = sdk_list_command.output()?;
        if sdks_output.status != Some(0) {
            return Err(format!(
                "Unable to list installed dotnet SDKs.\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&sdks_output.stdout),
                String::from_utf8_lossy(&sdks_output.stderr)
            ));
        }

        let stdout = String::from_utf8_lossy(&sdks_output.stdout);
        let installed_sdks = stdout
            .lines()
            .filter_map(|line| {
                let version = line.split_whitespace().next()?;
                let start = line.find('[')? + 1;
                let end = line.rfind(']')?;
                Some((version.to_string(), line[start..end].to_string()))
            })
            .collect::<Vec<_>>();

        if installed_sdks.is_empty() {
            return Err(format!(
                "Unable to parse installed dotnet SDKs from output: {}",
                stdout
            ));
        }

        let (sdk_version, sdk_base_path) = installed_sdks
            .last()
            .expect("installed_sdks is non-empty after explicit guard");

        let sdk_path = std::path::Path::new(&sdk_base_path)
            .join(sdk_version)
            .to_string_lossy()
            .to_string();
        Ok((sdk_path, sdk_version.clone()))
    }

    // Find the Razor Compiler DLL in the given SDK path
    fn find_razor_compiler_dll(sdk_path: &str) -> String {
        let dll_path = format!(
            "{}/Sdks/Microsoft.NET.Sdk.Razor/source-generators/Microsoft.CodeAnalysis.Razor.Compiler.dll",
            sdk_path
        );

        return dll_path;
    }

    // Find the Razor Design Time Targets file in the given SDK path
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

    fn install_or_update_razor(
        lsp_settings: LspSettings,
        language_server_id: &LanguageServerId,
        dotnet_command: &str,
        dotnet_env: Option<&EnvVars>,
    ) -> Result<Option<Vec<String>>, String> {
        let lsp_user_settings = match lsp_settings.settings {
            Some(settings) => settings,
            None => {
                return Ok(None); // no settings is also fine => no razor support
            }
        };

        let razor_root = lsp_user_settings["razor_source_repository_root"]
            .as_str()
            .map(|s| s.to_string());

        let razor_root_unwrapped = &match razor_root {
            None => return Ok(None), // no settings is also fine => no razor support
            Some(x) => x,
        };

        let (sdk_path, sdk_version) = Self::find_dotnet_sdk_path(dotnet_command, dotnet_env)?;

        let directory_exists = zed_extension_api::Command::new("test")
            .arg("-d")
            .arg(razor_root_unwrapped)
            .output()?;

        if directory_exists.status.unwrap() == 0 {
            zed_extension_api::set_language_server_installation_status(
                language_server_id,
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
            let razor_root_git_pull = zed_extension_api::process::Command::new("git")
                .arg("-C")
                .arg(razor_root_unwrapped)
                .arg("pull")
                .output()?;

            zed_extension_api::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::None,
            );

            if razor_root_git_pull.status.is_none() || razor_root_git_pull.status.unwrap() != 0 {
                println!("Unable to pull latest changes in razor repository. Offline?");
            }
        } else {
            // in this case we need to clone the repository
            zed_extension_api::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            let razor_root_clone = zed_extension_api::process::Command::new("git")
                .arg("clone")
                .arg("https://github.com/dotnet/razor")
                .arg(razor_root_unwrapped)
                .output()?;

            zed_extension_api::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::None,
            );

            if razor_root_clone.status.is_none() || razor_root_clone.status.unwrap() != 0 {
                return Err(format!("Unable to clone razor git repository. For this initial setup step, an internet connection is required."));
            }
        }

        let mut dotnet_build_command = new_dotnet_command(dotnet_command, dotnet_env)
                .arg("build")
                .arg(format!(
                    "{}/src/Razor/src/Microsoft.VisualStudioCode.RazorExtension/Microsoft.VisualStudioCode.RazorExtension.csproj",
                    razor_root_unwrapped
                ))
                .arg("--configuration")
                .arg("Release");
        let dotnet_build = dotnet_build_command.output()?;

        if dotnet_build.status.is_none() || dotnet_build.status.unwrap() != 0 {
            return Err(format!(
                "Unable to build razor extension.\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&dotnet_build.stdout),
                String::from_utf8_lossy(&dotnet_build.stderr)
            ));
        }

        // if someone knows a better way to get this dll, i'm all ears
        let sdk_version_short = format!(
            "net{}",
            sdk_version.split('.').take(2).collect::<Vec<_>>().join(".")
        );
        let razor_vscode_extension_path = format!("{}/artifacts/bin/Microsoft.VisualStudioCode.RazorExtension/Release/{}/Microsoft.VisualStudioCode.RazorExtension.dll", razor_root_unwrapped, sdk_version_short);

        let mut args = vec![];

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

        return Ok(Some(args));
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
                config_map.insert(key.clone(), value.clone());
            }
        }

        roslyn_config
    }
}

fn download_roslyn_server(
    dotnet_command: &str,
    dotnet_env: Option<&EnvVars>,
) -> Result<(), String> {
    let mut command = new_dotnet_command(dotnet_command, dotnet_env)
        .arg("tool")
        .arg("install")
        .arg("--global")
        .arg("roslyn-language-server")
        .arg("--prerelease")
        .arg("--source")
        .arg(
            "https://pkgs.dev.azure.com/azure-public/vside/_packaging/vs-impl/nuget/v3/index.json",
        );
    let output = command.output()?;
    if output.status != Some(0) {
        return Err(format!(
            "Unable to install roslyn-language-server.\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

fn update_roslyn_server(dotnet_command: &str, dotnet_env: Option<&EnvVars>) -> Result<(), String> {
    let mut command = new_dotnet_command(dotnet_command, dotnet_env)
        .arg("tool")
        .arg("update")
        .arg("roslyn-language-server")
        .arg("--global")
        .arg("--prerelease")
        .arg("--source")
        .arg(
            "https://pkgs.dev.azure.com/azure-public/vside/_packaging/vs-impl/nuget/v3/index.json",
        );
    let output = command.output()?;
    if output.status != Some(0) {
        return Err(format!(
            "Unable to update roslyn-language-server.\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}
