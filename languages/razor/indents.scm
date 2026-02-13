;; ============================================================================
;; Razor Indentation Rules
;; ============================================================================

;; Indent
[
  ;; Razor blocks
  (razor_block)
  (razor_if)
  (razor_else_if)
  (razor_else)
  (razor_switch)
  (razor_switch_case)
  (razor_switch_default)
  (razor_for)
  (razor_foreach)
  (razor_while)
  (razor_do_while)
  (razor_try)
  (razor_catch)
  (razor_finally)
  (razor_compound_using)
  (razor_lock)
  (razor_section)

  ;; C# blocks
  (block)
  (declaration_list)
  (accessor_list)
  (switch_body)
  (enum_member_declaration_list)

  ;; HTML elements
  (element)
] @indent

;; Outdent
[
  "}"
  "]"
  ")"
] @outdent

;; Branch points (else, else if, catch, finally, case, default)
[
  (razor_else_if)
  (razor_else)
  (razor_catch)
  (razor_finally)
  (razor_switch_case)
  (razor_switch_default)
] @branch
