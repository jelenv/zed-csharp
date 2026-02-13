;; ============================================================================
;; Razor Injections - Define language injections for C# and HTML content
;; ============================================================================

;; C# Code Blocks
(razor_block) @injection.content
(#set! injection.language "c_sharp")

((element) @injection.content
  (#set! injection.language "html")
  (#set! injection.combined))

;; Comments
(razor_comment) @injection.content
(#set! injection.language "comment")

(html_comment) @injection.content
(#set! injection.language "comment")

(comment) @injection.content
(#set! injection.language "comment")
