;; ============================================================================
;; Razor Code Outline/Structure
;; ============================================================================

;; Razor Directives
(razor_page_directive) @context
(razor_model_directive) @context
(razor_namespace_directive) @context

;; Razor Blocks (code and functions)
(razor_block) @context

;; Razor Sections
(razor_section
  (identifier) @name) @context

;; Classes
(class_declaration
  name: (identifier) @name) @context

;; Structs
(struct_declaration
  name: (identifier) @name) @context

;; Interfaces
(interface_declaration
  name: (identifier) @name) @context

;; Enums
(enum_declaration
  name: (identifier) @name) @context

;; Records
(record_declaration
  name: (identifier) @name) @context

;; Methods
(method_declaration
  name: (identifier) @name) @context

;; Local Functions
(local_function_statement
  name: (identifier) @name) @context

;; Properties
(property_declaration
  name: (identifier) @name) @context

;; Fields
(field_declaration
  (variable_declaration
    (variable_declarator
      (identifier) @name))) @context

;; Constructors
(constructor_declaration
  name: (identifier) @name) @context

;; Destructors
(destructor_declaration
  name: (identifier) @name) @context

;; Razor Control Structures
(razor_if) @context
(razor_switch) @context
(razor_for) @context
(razor_foreach) @context
(razor_while) @context
(razor_do_while) @context
(razor_try) @context

;; HTML Elements (major structural elements) - commented out due to hidden node
; ((element
;   (_tag_name) @name
;   (#match? @name "^(html|head|body|header|footer|main|nav|section|article|aside|div|form|table)$")) @context)
