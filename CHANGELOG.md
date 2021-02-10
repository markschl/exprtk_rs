# Change Log

## v0.1.0

The most important change regards `Expression::value()`, which now takes a 
mutable reference to `self`. This is important, because expression evaluation
can have side effects, variable assignments will modify the symbol table.

This has the consequence, that it isn't possible anymore to evaluate an 
expression if there are references to values (modifiable via `Cell` references);
intermediate modifications to the symbol table always require access by a 
variable ID.

In order to make the API more ergonomic and consistent, some changes were 
introduced (see documentation).

Also, `Expression`, `SymbolTable` and `StringValue` are now `Send` + `Sync`.
