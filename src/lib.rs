//! This crate provides bindings for the [ExprTk](http://www.partow.net/programming/exprtk/index.html)
//! library.
//!
//! For an overview of the data structures [see the ExprTk main page](http://www.partow.net/programming/exprtk/index.html).
//! While `exprtk-sys` maps most functions of the library to Rust, the high level bindings
//! were considerably simplified. Each [Expression](struct.Expression.html) owns a
//! [SymbolTable](struct.SymbolTable.html), they cannot be shared between different instances,
//! and multiple symbol tables per expression are not possible.
//!
//! Variables are owned by the `SymbolTable` instance. The functions for adding variables
//! ([add_variable()](exprtk/struct.SymbolTable.html#method.add_variable)), strings
//! ([add_stringvar()](exprtk/struct.SymbolTable.html#method.add_stringvar)), vectors
//! ([add_vector()](exprtk/struct.SymbolTable.html#method.add_vector)) all return
//! `usize`, which is a _variable ID_ representing the index in of the value in
//! an internal data structure. It can be used to later get symbol values and modify them.
//! Scalars are either modified via mutable references, or via `std::cell::Cell` types without
//! the requirement of mutable access to the `SymbolTable`.
//! Strings are changed using [set_string()](exprtk/struct.SymbolTable.html#method.set_string),
//! which requires mutable access.
//! Since access and mutation through variable IDs requires a bounds check, these operations
//! are slower than direct modification through pointers, as done in C++. The performance impact
//! is naturally more severe for small expressions with fast running times, but seems not too
//! problematic in most cases. Run `cargo bench` to see the impact (compare with unsafe variant).
//! For each data type (scalars, strings and vectors), access IDs start at zero
//! and are incremented on addition of new variables of the given type.
//!
//! As there is no guarantee that `double` is always `f64`, the `c_double` type is used all
//! over the library. Other precisions are currently not supported.
//!
//! ExprTk does not handle non-ASCII encodings, therefore variable names and formulae are
//! checked for non-ASCII characters or null bytes and will fail with an error.
//!
//! # Examples:
//!
//! This code corresponds to the [example 1](http://www.partow.net/programming/exprtk/index.html#simpleexample01)
//! in the ExprTk documentation:
//!
//! ```no_run
//! use exprtk_rs::*;
//!
//! let expression_string = "clamp(-1.0,sin(2 * pi * x) + cos(x / 2 * pi),+1.0)";
//!
//! let mut symbol_table = SymbolTable::new();
//! symbol_table.add_constants();
//! let var_id = symbol_table.add_variable("x", 0.).unwrap().unwrap();
//!
//! let mut expression = Expression::new(expression_string, symbol_table).unwrap();
//!
//! // this value is a reference to a std::cell::Cell that can be changed
//! expression.symbols().value_cell(var_id).set(-5.);
//!
//! while expression.symbols().value(var_id) <= 5. {
//!     let y = expression.value();
//!     println!("{}\t{}", expression.symbols().value(var_id), y);
//!     *expression.symbols_mut().value_mut(var_id) += 0.001;
//! }
//! ```
//!
//! # Unknown variables
//!
//! Unknown variables encountered in an expression can be automatically added to the symbol table.
//! The function `Expression::parse_vars` will return a `Vec` containing the newly added variable
//! names and their variable IDs.
//! This works only for regular variables, not for strings or vectors.
//!
//! ```
//! use exprtk_rs::*;
//!
//! let expr_string = "a*x^2 + b*x + c";
//!
//! let (mut expr, unknown_vars) = Expression::parse_vars(expr_string, SymbolTable::new()).unwrap();
//!
//! assert_eq!(
//!     unknown_vars,
//!     vec![("a".to_string(), 0), ("x".to_string(), 1), ("b".to_string(), 2), ("c".to_string(), 3)]
//! );
//!
//! // modify the values
//! expr.symbols().value_cell(0).set(2.); // a
//! expr.symbols().value_cell(2).set(3.); // b
//! expr.symbols().value_cell(3).set(1.); // c
//! expr.symbols().value_cell(1).set(5.); // x
//!
//! assert_eq!(expr.value(), 66.);
//! ```
//!
//! # Example using strings
//!
//! ```
//! use exprtk_rs::*;
//!
//! let mut symbol_table = SymbolTable::new();
//! let s1_id = symbol_table.add_stringvar("s1", "Hello").unwrap().unwrap();
//! let s2_id = symbol_table.add_stringvar("s2",  "world!").unwrap().unwrap();
//!
//! // concatenation
//! let mut expr = Expression::new("s1 + ' ' + s2 == 'Hello world!'", symbol_table).unwrap();
//! // a boolean `true` is represented by `1`
//! assert_eq!(expr.value(), 1.);
//!
//! // Modifying a string
//! expr.symbols_mut().set_string(s1_id, "");
//! assert_eq!(expr.value(), 0.);
//! ```
//!
//! # Functions
//!
//! There is currently the possibility to add functions/closures with up to ten scalar arguments.
//! Example:
//!
//! ```
//! use exprtk_rs::*;
//!
//! let mut symbol_table = SymbolTable::new();
//! symbol_table.add_func2("add", |x, y| x + y);
//! symbol_table.add_variable("x", 1.).unwrap();
//!
//! let mut expr = Expression::new("add(x, 1)", symbol_table).unwrap();
//! assert_eq!(expr.value(), 2.);
//! ```

#[macro_use]
extern crate enum_primitive;

pub use error::*;
pub use exprtk::*;
pub use libc::c_double;

macro_rules! string_from_ptr {
    ($s:expr) => {
        CStr::from_ptr($s).to_string_lossy().into_owned()
    };
}

mod error;
mod exprtk;

#[cfg(test)]
mod tests;
