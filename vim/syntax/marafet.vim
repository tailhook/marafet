" Vim syntax file
" Language: Marafet
" Maintainer:Paul Colomiets

if exists("b:current_syntax")
  finish
endif

syn keyword marafetToplevels html css import from
syn keyword marafetFlow if elif else for
syn keyword marafetOperator and or not
syn keyword marafetSpecial store link
syn match marafetComment "#.*$" contains=marafetTodo
syn region marafetString start='"' end='"'
syn region marafetString start="'" end="'"

hi def link marafetToplevels PreProc
hi def link marafetFlow Conditional
hi def link marafetSpecial Special
hi def link marafetComment Comment
hi def link marafetString Constant
hi def link marafetOperator Operator
