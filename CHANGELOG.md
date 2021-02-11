# Change Log

## v0.1.0

Updates strongly encouraged!

The most important change regards `Expression::value()`, which now takes a 
mutable reference to `self`. This is important, because expression evaluation
can have side effects, variable assignments will modify the symbol table.

This has the consequence, that it isn't possible anymore to evaluate an 
expression if there are references to values (modifiable via `Cell` references);
intermediate modifications to the symbol table always require access by a 
variable ID.

In order to make the API more ergonomic and consistent, some changes were 
introduced (see documentation).

* Several method names were changed, some added (for mutable access)
* `Expression`, `SymbolTable` and `StringValue` are now `Send` + `Sync`
* Methods for adding constants did not take `self` as mutable
