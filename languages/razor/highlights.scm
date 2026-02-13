;; Inherit C# highlighting
; (inherits: c_sharp)

;; ============================================================================
;; Razor-specific Highlighting
;; ============================================================================

;; Razor Markers and Keywords
[
  "at_page"
  "at_using"
  "at_model"
  "at_inject"
  "at_implements"
  "at_inherits"
  "at_layout"
  "at_attribute"
  "at_typeparam"
  "at_namespace"
  "at_preservewhitespace"
  "at_rendermode"
  "at_section"
  "at_block"
  "at_if"
  "at_switch"
  "at_for"
  "at_foreach"
  "at_while"
  "at_do"
  "at_try"
  "at_await"
  "at_lock"
  "at_implicit"
  "at_explicit"
  "at_colon_transition"
  "at_at_escape"
] @keyword

;; Razor Directives
(razor_page_directive) @keyword.directive
(razor_using_directive) @keyword.directive
(razor_model_directive) @keyword.directive
(razor_inject_directive) @keyword.directive
(razor_implements_directive) @keyword.directive
(razor_inherits_directive) @keyword.directive
(razor_layout_directive) @keyword.directive
(razor_attribute_directive) @keyword.directive
(razor_typeparam_directive) @keyword.directive
(razor_namespace_directive) @keyword.directive
(razor_preservewhitespace_directive) @keyword.directive
(razor_rendermode_directive) @keyword.directive

;; Razor Render Modes
(razor_rendermode) @constant.builtin

;; Razor Comments
(razor_comment) @comment

;; Razor Expressions
(razor_implicit_expression) @variable
(razor_explicit_expression) @variable
(razor_await_expression) @keyword

;; Razor Control Structures
(razor_if) @keyword.control.conditional
(razor_else_if) @keyword.control.conditional
(razor_else) @keyword.control.conditional
(razor_switch) @keyword.control.conditional
(razor_switch_case) @keyword.control.conditional
(razor_switch_default) @keyword.control.conditional
(razor_for) @keyword.control.repeat
(razor_foreach) @keyword.control.repeat
(razor_while) @keyword.control.repeat
(razor_do_while) @keyword.control.repeat
(razor_try) @keyword.control.exception
(razor_catch) @keyword.control.exception
(razor_finally) @keyword.control.exception

;; Razor Blocks
(razor_block) @keyword

;; Razor Section
(razor_section
  (identifier) @function)

;; Razor Compound Using
(razor_compound_using) @keyword

;; Razor Lock
(razor_lock) @keyword

;; Razor Condition
(razor_condition) @keyword

;; ============================================================================
;; HTML Elements
;; ============================================================================

;; HTML Tags
(element
  "<" @tag.delimiter
  ">" @tag.delimiter
  "</" @tag.delimiter
  "/>" @tag.delimiter)

;; HTML Attributes (using node types without hidden nodes)
(element) @tag

;; Razor HTML Attributes
(razor_html_attribute
  (razor_attribute_name) @attribute)

(razor_attribute_name) @attribute

;; Razor Attribute Modifiers
(razor_attribute_modifier) @attribute

;; Razor Attribute Values
(razor_attribute_value) @string

;; HTML Comments
(html_comment) @comment

;; ============================================================================
;; C# Inherited Rules (Key Highlights)
;; ============================================================================

;; Identifiers
(identifier) @variable

;; Types
(type_declaration
  name: (identifier) @type)

(interface_declaration
  name: (identifier) @type)

(class_declaration
  name: (identifier) @type)

(struct_declaration
  name: (identifier) @type)

(enum_declaration
  name: (identifier) @type)

(record_declaration
  name: (identifier) @type)

;; Methods
(method_declaration
  name: (identifier) @function)

(local_function_statement
  name: (identifier) @function)

;; Properties
(property_declaration
  name: (identifier) @property)

;; Parameters
(parameter
  name: (identifier) @parameter)

;; Built-in Types
(predefined_type) @type.builtin

;; Literals
[
  (integer_literal)
  (real_literal)
] @number

[
  (character_literal)
  (string_literal)
  (raw_string_literal)
  (verbatim_string_literal)
  (interpolated_string_expression)
] @string

(string_literal_content) @string
(escape_sequence) @string.escape

[
  (boolean_literal)
  (null_literal)
] @constant.builtin

;; Keywords
[
  "if"
  "else"
  "switch"
  "case"
  "default"
] @keyword.control.conditional

[
  "for"
  "foreach"
  "while"
  "do"
  "in"
] @keyword.control.repeat

[
  "try"
  "catch"
  "finally"
  "throw"
] @keyword.control.exception

[
  "break"
  "continue"
  "goto"
  "return"
  "yield"
] @keyword.control.return

[
  "using"
  "namespace"
] @keyword.control.import

[
  "async"
  "await"
] @keyword.coroutine

[
  "new"
] @keyword.operator

[
  "class"
  "struct"
  "interface"
  "enum"
  "record"
] @keyword.type

[
  "public"
  "private"
  "protected"
  "internal"
  "static"
  "virtual"
  "override"
  "abstract"
  "sealed"
  "readonly"
  "const"
  "volatile"
  "extern"
  "unsafe"
  "partial"
] @keyword.modifier

[
  "this"
  "base"
] @variable.builtin

[
  "var"
] @keyword

;; Operators
[
  "+"
  "-"
  "*"
  "/"
  "%"
  "="
  "=="
  "!="
  "<"
  ">"
  "<="
  ">="
  "&&"
  "||"
  "!"
  "&"
  "|"
  "^"
  "~"
  "<<"
  ">>"
  "+="
  "-="
  "*="
  "/="
  "%="
  "&="
  "|="
  "^="
  "<<="
  ">>="
  "??"
  "??="
  "=>"
  "?"
  ":"
  "++"
  "--"
] @operator

;; Punctuation
[
  ";"
  ","
  "."
] @punctuation.delimiter

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

;; Comments
(comment) @comment

;; Preprocessor
[
  (preproc_if)
  (preproc_elif)
  (preproc_else)
  (preproc_define)
  (preproc_undef)
  (preproc_region)
  (preproc_endregion)
  (preproc_pragma)
  (preproc_nullable)
  (preproc_error)
  (preproc_warning)
  (preproc_line)
] @keyword.directive
