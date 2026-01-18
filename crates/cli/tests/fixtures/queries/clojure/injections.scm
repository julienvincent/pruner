((list_lit
  ((sym_lit) @def-type
   (sym_lit) @def-name
   (str_lit)? @docstring @injection.content)
   (map_lit)?

   [
    (vec_lit)
    (list_lit (vec_lit))+
   ])

  (#match? @def-type "^(defn-?|defmacro)$")
  (#offset! @injection.content 0 1 0 -1)
  (#escape! @injection.content "\"")
  (#set! injection.language "markdown"))

((str_lit)  @injection.content
  (#match? @injection.content "^\"(SELECT|CREATE|ALTER|UPDATE|DROP|INSERT)")
  (#offset! @injection.content 0 1 0 -1)
  (#escape! @injection.content "\"")
  (#set! injection.language "sql"))
