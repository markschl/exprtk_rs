use std::ops::Drop;
use std::ffi::*;
use std::ptr;
use std::mem;
use std::fmt;
use std::cell::Cell;

use libc::{c_char, size_t, c_double, c_void};
use exprtk_sys::*;
use super::*;

// Sending pointers to CExpression and CSymbolTable
// around should be safe. Calls to methods with non-mutable
// access to self should have no side-effects
unsafe impl Send for Expression {}
unsafe impl Send for SymbolTable {}
unsafe impl Sync for Expression {}
unsafe impl Sync for SymbolTable {}


fn c_string(s: &str) -> Result<CString, InvalidName> {
    CString::new(s).map_err(|_| InvalidName(s.to_string()))
}


#[derive(Debug)]
struct Parser(*mut CParser);

impl Parser {
    pub fn new() -> Parser {
        unsafe { Parser(parser_new()) }
    }

    fn formula_to_cstring(s: &str) -> Result<CString, ParseError> {
        c_string(s).map_err(From::from)
    }

    pub fn compile(&self, string: &str, expr: &Expression) -> Result<(), ParseError> {
        let formula = Self::formula_to_cstring(string)?;
        unsafe {
            if !parser_compile(self.0, formula.as_ptr(), expr.expr) {
                return Err(self.get_err());
            }
        }
        Ok(())
    }

    pub fn compile_resolve<F, S>(
        &self,
        string: &str,
        expr: &mut Expression,
        mut func: F,
    ) -> Result<(), ParseError>
    where F: FnMut(&str, &mut SymbolTable) -> Result<(), S>,
          S: AsRef<str>
    {
        let formula = Self::formula_to_cstring(string)?;
        let expr_ptr = expr.expr;
        let symbols = expr.symbols_mut();
        let mut user_data = (symbols, &mut func);
        unsafe {
            let r = parser_compile_resolve(self.0, formula.as_ptr(), expr_ptr, wrapper::<F, S>, &mut user_data as *const _ as *mut c_void);
            if !r {
                return Err(self.get_err());
            }
        };

        extern fn wrapper<F, S>(c_name: *const c_char, user_data: *mut c_void) -> *const c_char
        where F: FnMut(&str, &mut SymbolTable) -> Result<(), S>,
        S: AsRef<str>
        {
            let (ref mut symbols, ref mut opt_f) = unsafe {
                &mut *(user_data as *mut (&mut SymbolTable, Option<&mut F>))
            };
            let name = unsafe { CStr::from_ptr(c_name).to_str().unwrap() };
            opt_f.as_mut().map(|ref mut f| {
                if let Err(e) = f(name, symbols) {
                    return CString::new(e.as_ref()).unwrap().into_raw() as *const c_char
                }
                ptr::null() as *const c_char
            }).unwrap()
        }
        Ok(())
    }

    fn get_err(&self) -> ParseError {
        unsafe { ParseError::from_c_err(self.0) }
            .expect("Compiler notified about error, but there is none.")
    }
}

impl Drop for Parser {
    fn drop(&mut self) {
        unsafe { parser_destroy(self.0) };
    }
}



pub struct Expression {
    expr: *mut CExpression,
    string: String,
    symbols: SymbolTable,
}


impl Expression {
    /// Compiles a new `Expression`. Missing variables will lead to a
    /// `exprtk::ParseError`.
    ///
    /// # Example:
    /// The above example melts down to this:
    ///
    /// ```
    /// use exprtk_rs::*;
    ///
    /// let mut symbol_table = SymbolTable::new();
    /// symbol_table.add_variable("a", 2.).unwrap();
    /// let mut expr = Expression::new("a + 1", symbol_table).unwrap();
    /// assert_eq!(expr.value(), 3.);
    /// ```
    pub fn new(string: &str, symbols: SymbolTable) -> Result<Expression, ParseError> {
        let parser = Parser::new();
        let e = Expression {
            expr: unsafe { expression_new() },
            string: string.to_string(),
            symbols,
        };
        e.register_symbol_table();
        parser.compile(string, &e)?;
        Ok(e)
    }

    /// Compiles a new `Expression` like `Expression::new`. In addition, if
    /// unknown variables are encountered, they are automatically added an internal `SymbolTable`
    /// and initialized with `0.`. Their names and variable IDs are returned as tuples together
    /// with the new `Expression` instance.
    pub fn parse_vars(string: &str, symbols: SymbolTable) -> Result<(Expression, Vec<(String, usize)>), ParseError> {
        let mut vars = vec![];
        let e = Expression::handle_unknown(string, symbols, |name, symbols| {
            let var_id = symbols
                .add_variable(name, 0.)
                .map_err(|_| "invalid name.")?
                .unwrap();
            vars.push((name.to_string(), var_id));
            Ok(())
        })?;
        Ok((e, vars))
    }

    /// Handles unknown variables like `Expression::parse_vars()` does, but instead of creating
    /// a new `SymbolTable`, an existing one can be supplied, which may already have some
    /// variables defined. The variables are handled in a closure, which can register
    /// the names as variables, constants, strings or vectors to the supplied symbol table.
    ///
    /// # Example
    ///
    /// ```
    /// use exprtk_rs::*;
    ///
    /// let formula = "s_string[] + a + b";
    /// let mut expr = Expression::handle_unknown(formula, SymbolTable::new(), |name, sym| {
    ///     if name.starts_with("s_") {
    ///         sym.add_stringvar(name, "string").unwrap();
    ///     } else {
    ///         sym.add_variable(name, 1.).unwrap();
    ///     }
    ///     Ok(())
    /// }).unwrap();
    ///
    /// assert_eq!(expr.value(), 8.);
    /// ```
    /// **Note**: this function is very flexible, but can cause problems if **not** registering
    /// anything for a given name. In that case, the same variable name will be brought up again
    /// and again, infinite loop, ultimately resulting in a stack overflow.
    pub fn handle_unknown<F>(
        string: &str,
        symbols: SymbolTable,
        func: F,
    ) -> Result<Expression, ParseError>
    where F: FnMut(&str, &mut SymbolTable) -> Result<(), String>
    {
        let parser = Parser::new();
        let mut e = Expression {
            expr: unsafe { expression_new() },
            string: string.to_string(),
            symbols,
        };
        e.register_symbol_table();

        parser.compile_resolve(string, &mut e, func)?;

        Ok(e)
    }

    fn register_symbol_table(&self) {
        unsafe {
            expression_register_symbol_table(self.expr, self.symbols.sym);
        }
    }

    /// Calculates the value of the expression. Returns `NaN` if the expression was not yet
    /// compiled.
    /// 
    /// *Note*: This method requires mutable access to the underlying expression
    /// object, since executing an expression can have side-effects. Variables
    /// in the symbol table of the expression can be changed or added.
    pub fn value(&mut self) -> c_double {
        unsafe { expression_value(self.expr) }
    }

    /// Returns a reference to the symbol table owned by the `Expression`
    #[inline]
    pub fn symbols(&self) -> &SymbolTable {
        &self.symbols
    }

    /// Returns a mutable reference to the symbol table owned by the `Expression`
    #[inline]
    pub fn symbols_mut(&mut self) -> &mut SymbolTable {
        &mut self.symbols
    }
}


impl Drop for Expression {
    fn drop(&mut self) {
        unsafe { expression_destroy(self.expr) };
    }
}


impl fmt::Debug for Expression {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Expression {{ string: {}, symbols: {:?} }}",
            self.string,
            self.symbols
        )
    }
}

impl Clone for Expression {
    fn clone(&self) -> Expression {
        Expression::new(&self.string, self.symbols.clone()).unwrap()
    }
}

struct FuncData {
    name: String,
    cpp_func: *mut c_void,
    rust_closure: *mut c_void,
    clone_func: fn(&str, *mut c_void, &mut SymbolTable) -> Result<bool, InvalidName>,
    free_cpp_func: unsafe extern "C" fn(*mut c_void),
    free_closure_func: fn(*mut c_void),
}

/// `SymbolTable` holds different variables. There are three types of variables:
/// Numberic variables, strings and numeric vectors of fixed size. (see
/// [the documentation](https://github.com/ArashPartow/exprtk/blob/f32d2b4bbb640ea4732b8a7fce1bd9717e9c998b/readme.txt#L643)).
/// Many but not all of the methods of the [ExprTk symbol_table](http://partow.net/programming/exprtk/doxygen/classexprtk_1_1symbol__table.html)
/// were implemented, and the API is sometimes different.
pub struct SymbolTable {
    sym: *mut CSymbolTable,
    values: Vec<Cell<c_double>>,
    strings: Vec<StringValue>,
    vectors: Vec<Box<[c_double]>>,
    funcs: Vec<FuncData>,
}

impl SymbolTable {
    pub fn new() -> SymbolTable {
        SymbolTable {
            sym: unsafe { symbol_table_new() },
            values: vec![],
            strings: vec![],
            vectors: vec![],
            funcs: vec![],
        }
    }

    pub fn add_constant(&mut self, name: &str, value: c_double) -> Result<bool, InvalidName> {
        let c_name = c_string(name)?;
        let rv = unsafe { symbol_table_add_constant(self.sym, c_name.as_ptr(), value) };
        let added = self.validate_added(name, rv, ())?;
        Ok(added.is_some())
    }

    /// Adds a new variable. Returns the variable ID that can later be used for `set_value`
    /// or `None` if a variable with the same name was already present.
    /// The behavior of this function differs from
    /// [the one of the underlying library](http://www.partow.net/programming/exprtk/doxygen/classexprtk_1_1symbol__table.html)
    /// by not providing the (optional) `is_constant` option. Use `add_constant()` instead.
    pub fn add_variable(&mut self, name: &str, value: c_double) -> Result<Option<usize>, InvalidName> {
        let i = self.values.len();
        self.values.push(Cell::new(value));
        let cell = self.values.last().unwrap();
        let c_name = c_string(name)?;
        let rv =
            unsafe { symbol_table_add_variable(self.sym, c_name.as_ptr(), cell.as_ptr(), false) };
        self.validate_added(name, rv, i)
    }

    /// Returns the value of a variable given its variable ID
    #[inline]
    pub fn value(&self, var_id: usize) -> c_double {
        self.values[var_id].get()
    }

    /// Returns a mutable reference to the value of a registered variable.
    /// 
    /// # Panics
    /// 
    /// This function will panic if the `var_id` refers to an unknown variable,
    /// more specifically if it is larger than the largest variable ID.
    ///
    /// # Example:
    /// ```
    /// use std::f64::consts::PI;
    /// use exprtk_rs::*;
    ///
    /// let mut symbol_table = SymbolTable::new();
    /// let x_id = symbol_table.add_variable("x", 0.).expect("Invalid name").expect("Already present");
    /// let mut expr = Expression::new("sin(x)", symbol_table).expect("Compile error");
    /// assert_eq!(expr.value(), 0.);
    ///
    /// let mut x = 0.;
    /// while expr.symbols().value(x_id) <= 2. * PI {
    ///     *expr.symbols_mut().value_mut(x_id) += 0.1;
    ///     let y = expr.value();
    /// }
    /// ```
    #[inline]
    pub fn value_mut(&mut self, var_id: usize) -> &mut c_double {
        self.values[var_id].get_mut()
    }

    /// Returns the value of a registered variable as modifiable `std::cell::Cell`.
    /// This is an alternative access to `value_mut` and allows changing the values
    /// easily. If the reference to the `Cell` is kept around, its value can be
    /// changed multiple times.
    /// 
    /// # Panics
    /// 
    /// This function will panic if the `var_id` refers to an unknown variable,
    /// more specifically if it is larger than the largest variable ID.
    ///
    /// # Example:
    /// ```
    /// use exprtk_rs::*;
    ///
    /// let mut symbol_table = SymbolTable::new();
    /// let id = symbol_table.add_variable("a", 2.).expect("Invalid name").expect("Already present");
    /// let mut expr = Expression::new("a - 1", symbol_table).expect("Compile error");
    /// assert_eq!(expr.value(), 1.);
    ///
    /// let value = expr.symbols().value_cell(id);
    /// value.set(4.);
    /// assert_eq!(expr.value(), 3.);
    /// ```
    #[inline]
    pub fn value_cell(&self, var_id: usize) -> &Cell<c_double> {
        &self.values[var_id]
    }

    /// Returns the value of a variable (whether constant or not)
    /// 
    /// # Panics
    /// 
    /// This function will panic if the `name` refers to an unknown variable.
    #[inline]
    pub fn value_from_name(&self, name: &str) -> Result<c_double, InvalidName> {
        let c_name = c_string(name)?;
        let var_ref = unsafe { symbol_table_variable_ref(self.sym, c_name.as_ptr()).as_ref().cloned() };
        Ok(var_ref.expect("Unknown variable name"))
    }

    /// Adds a new string variable. Returns the variable ID that can later be used for `set_string`
    /// or `None` if a variable with the same name was already present.
    pub fn add_stringvar(&mut self, name: &str, text: &str) -> Result<Option<usize>, InvalidName> {
        let i = self.strings.len();
        let s = StringValue::new(text);
        self.strings.push(s);

        let c_name = c_string(name)?;
        let rv = unsafe {
            symbol_table_add_stringvar(self.sym, c_name.as_ptr(), self.strings[i].0, false)
        };

        let res = self.validate_added(name, rv, i);
        if res.is_err() {
            self.strings.pop();
        }
        res
    }

    #[inline]
    pub fn set_string(&mut self, var_id: usize, text: &str) -> bool {
        if let Some(s) = self.string_mut(var_id) {
            s.set(text);
            return true;
        }
        false
    }

    /// Returns an immutable reference to a string variable given its ID.
    #[inline]
    pub fn string(&self, var_id: usize) -> Option<&StringValue> {
        self.strings.get(var_id)
    }

    /// Returns a mutable reference to a string variable given its ID.
    #[inline]
    pub fn string_mut(&mut self, var_id: usize) -> Option<&mut StringValue> {
        self.strings.get_mut(var_id)
    }

    /// Adds a new vector variable. Returns the variable ID that can later be used for `vector`
    /// or `None` if a variable with the same name was already present.
    pub fn add_vector(&mut self, name: &str, vec: &[c_double]) -> Result<Option<usize>, InvalidName> {
        let i = self.vectors.len();
        let l = vec.len();
        self.vectors.push(vec.to_vec().into_boxed_slice());

        let c_name = c_string(name)?;
        let rv = unsafe {
            symbol_table_add_vector(self.sym, c_name.as_ptr(), self.vectors[i].as_ptr(), l)
        };

        let res = self.validate_added(name, rv, i);
        if res.is_err() {
            self.vectors.pop();
        }
        res
    }

    /// Returns an immutable reference to a vector given its variable ID.
    #[inline]
    pub fn vector(&self, var_id: usize) -> Option<&[c_double]> {
        self.vectors.get(var_id).map(|v| &**v)
    }

    /// Returns an mutable reference to a vector given its variable ID.
    #[inline]
    pub fn vector_mut(&mut self, var_id: usize) -> Option<&mut [c_double]> {
        self.vectors.get_mut(var_id).map(|v| &mut **v)
    }
    
    /// Returns a reference to a vector given its variable ID. The values are of the type
    /// `std::cell::Cell`, and can thus be modified without mutable access to `SymbolTable`
    #[inline]
    pub fn vector_of_cells(&self, var_id: usize) -> Option<&[Cell<c_double>]> {
        self.vectors.get(var_id).map(|v| unsafe {
            // Code equivalent to Cell::from_mut(v).as_slice_of_cells(), could be used once stable, but
            // would require mutable access to self.vectors, which we don't have here.
            // Alternatively, self.vectors could hold slices of Cells itself. However,
            // SymbolTable::value_vector would then require unsafe code to convert it back.
            // Therefore, using unsafe code here
            let cell_slice = &*(&**v as *const [c_double] as *const Cell<[c_double]>);
            // code from Cell::as_slice_of_cells(), replace when stable
            &*(cell_slice as *const Cell<[c_double]> as *const [Cell<c_double>])
        })
    }

    // Validate result of adding variable / string /...
    // add_variable() does three checks, and any of them failing leads to `false`.
    // 1. symbol table sanity
    // 2. valid name?
    // 3. symbol exists already?
    // Here, we want to distinguish the results of the three checks.
    fn validate_added<O>(&self, name: &str, result: bool, out: O) -> Result<Option<O>, InvalidName> {
        if !result {
            let valid = unsafe { symbol_table_valid(self.sym) };
            if !valid {
                panic!("Bug: SymbolTable state invalid!");
            }
            // We can't directly validate a name (valid_symbol is private),
            // but we can check if the variable was added
            if !self.symbol_exists(name).unwrap() {
                return Err(InvalidName(name.to_string()));
            }
            // This must be case 3)
            assert_eq!(self.symbol_exists(name), Ok(true));
            return Ok(None);
        }
        Ok(Some(out))
    }

    fn get_var_ptr(&self, name: &str) -> Result<Option<*mut c_double>, InvalidName> {
        let c_name = c_string(name)?;
        let rv = unsafe { symbol_table_variable_ref(self.sym, c_name.as_ptr()) };
        let rv = if rv.is_null() { None } else { Some(rv) };
        Ok(rv)
    }

    /// Returns the 'ID' of a variable or None if not found.
    /// The function will return `Err(InvalidName)` if the name is not entirely
    /// composed of ASCII characters.
    pub fn get_var_id(&self, name: &str) -> Result<Option<usize>, InvalidName> {
        self.get_var_ptr(name).map(|opt_ptr| {
            opt_ptr.and_then(|ptr| self.values.iter().position(|c| c.as_ptr() == ptr))
        })
    }

    /// Returns the 'ID' of a string or None if not found.
    /// The function will return `Err(InvalidName)` if the name is not entirely
    /// composed of ASCII characters.
    pub fn get_string_id(&self, name: &str) -> Result<Option<usize>, InvalidName> {
        let c_name = c_string(name)?;
        let ptr = unsafe { symbol_table_stringvar_ref(self.sym, c_name.as_ptr()) };
        let rv = if ptr.is_null() {
            None
        } else {
            self.strings.iter().position(|s| s.0 == ptr)
        };
        Ok(rv)
    }

    /// Returns the 'ID' of a vector or None if not found.
    /// The function will return `Err(InvalidName)` if the name is not entirely
    /// composed of ASCII characters.
    pub fn get_vec_id(&self, name: &str) -> Result<Option<usize>, InvalidName> {
        let c_name = c_string(name)?;
        let ptr = unsafe { symbol_table_vector_ptr(self.sym, c_name.as_ptr()) };
        let rv = if ptr.is_null() {
            None
        } else {
            self.vectors.iter().position(|v| v.as_ptr() == ptr)
        };
        Ok(rv)
    }

    pub fn clear_variables(&mut self) {
        self.values.clear();
        unsafe { symbol_table_clear_variables(self.sym) }
    }

    pub fn clear_strings(&mut self) {
        self.strings.clear();
        unsafe { symbol_table_clear_strings(self.sym) }
    }

    pub fn clear_vectors(&mut self) {
        self.vectors.clear();
        unsafe { symbol_table_clear_vectors(self.sym) }
    }

    pub fn clear_local_constants(&mut self) {
        unsafe { symbol_table_clear_local_constants(self.sym) }
    }

    pub fn clear_functions(&mut self) {
        unsafe { symbol_table_clear_functions(self.sym) }
    }

    pub fn variable_count(&self) -> usize {
        unsafe { symbol_table_variable_count(self.sym) as usize }
    }

    pub fn stringvar_count(&self) -> usize {
        unsafe { symbol_table_stringvar_count(self.sym) as usize }
    }

    pub fn vector_count(&self) -> usize {
        unsafe { symbol_table_vector_count(self.sym) as usize }
    }

    pub fn function_count(&self) -> usize {
        unsafe { symbol_table_function_count(self.sym) as usize }
    }

    pub fn add_constants(&self) -> bool {
        unsafe { symbol_table_add_constants(self.sym) }
    }

    pub fn add_pi(&self) -> bool {
        unsafe { symbol_table_add_pi(self.sym) }
    }

    pub fn add_epsilon(&self) -> bool {
        unsafe { symbol_table_add_epsilon(self.sym) }
    }

    pub fn add_infinity(&self) -> bool {
        unsafe { symbol_table_add_infinity(self.sym) }
    }

    pub fn get_variable_names(&self) -> Vec<String> {
        unsafe {
            let l = symbol_table_get_variable_list(self.sym);
            let out = (*l)
                .get_slice()
                .iter()
                .map(|s| string_from_ptr!(*s))
                .collect();
            string_array_free(l);
            out
        }
    }

    pub fn get_stringvar_names(&self) -> Vec<String> {
        unsafe {
            let l = symbol_table_get_stringvar_list(self.sym);
            let out = (*l)
                .get_slice()
                .iter()
                .map(|s| string_from_ptr!(*s))
                .collect();
            string_array_free(l);
            out
        }
    }

    pub fn get_vector_names(&self) -> Vec<String> {
        unsafe {
            let l = symbol_table_get_vector_list(self.sym);
            let out = (*l)
                .get_slice()
                .iter()
                .map(|s| string_from_ptr!(*s))
                .collect();
            string_array_free(l);
            out
        }
    }

    pub fn symbol_exists(&self, name: &str) -> Result<bool, InvalidName> {
        let c_name = c_string(name)?;
        let rv = unsafe { symbol_table_symbol_exists(self.sym, c_name.as_ptr()) };
        Ok(rv)
    }

    pub fn is_constant_node(&self, name: &str) -> Result<bool, InvalidName> {
        let c_name = c_string(name)?;
        let rv = unsafe { symbol_table_is_constant_node(self.sym, c_name.as_ptr()) };
        Ok(rv)
    }

    pub fn is_constant_string(&self, name: &str) -> Result<bool, InvalidName> {
        let c_name = c_string(name)?;
        let rv = unsafe { symbol_table_is_constant_string(self.sym, c_name.as_ptr()) };
        Ok(rv)
    }
}

macro_rules! func_impl {
    ($name:ident, $sys_func:ident, $clone_func:ident, $free_closure:ident, $free_cpp_func:ident,
        $($x:ident: $ty:ty),*) => {
        impl SymbolTable {
            /// Add a function. Returns `true` if the function was added / `false`
            /// if the name was already present.
            pub fn $name<F>(&mut self, name: &str, func: F) -> Result<bool, InvalidName>
                where F: Fn($($ty),*) -> c_double + Clone
            {
                extern fn wrapper<F>(closure: *mut c_void, $($x: $ty),*) -> c_double
                    where F: Fn($($ty),*) -> c_double {
                    unsafe {
                        let opt_closure: Option<Box<F>> = mem::transmute(closure);
                        opt_closure.map(|f| f($($x),*)).unwrap()
                    }
                }

                let func_box = Box::new(func);
                let func_ptr = Box::into_raw(func_box) as *mut _ as *mut c_void;

                let c_name = c_string(name)?;
                let result = unsafe {
                    $sys_func(self.sym, c_name.as_ptr(), wrapper::<F>, func_ptr)
                };

                let is_new = self.validate_added(name, result.0, ())?.is_some();
                if is_new {
                    self.funcs.push(FuncData {
                        name: name.to_string(),
                        cpp_func: result.1,
                        rust_closure: func_ptr,
                        clone_func: $clone_func::<F>,
                        free_cpp_func: $free_cpp_func,
                        free_closure_func: $free_closure::<F>,
                    });
                }
                Ok(is_new)
            }
        }

        fn $clone_func<F>(name: &str, closure_ptr: *mut c_void, new_symbols: &mut SymbolTable)
        -> Result<bool, InvalidName>
        where F: Fn($($ty),*) -> c_double + Clone
        {
            let mut opt_closure: Option<Box<F>> = unsafe { mem::transmute(closure_ptr) };
            let res = new_symbols.$name(name, *opt_closure.as_mut().unwrap().clone());
            mem::forget(opt_closure); // prevent destruction
            res
        }

        fn $free_closure<F>(closure_ptr: *mut c_void)
        where F: Fn($($ty),*) -> c_double + Clone
        {
            let _: Option<Box<F>> = unsafe { mem::transmute(closure_ptr) };
        }
    }
}

func_impl!(add_func1, symbol_table_add_func1, clone_func1,
    free_func_closure1, symbol_table_free_func1,
    a: c_double);
func_impl!(add_func2, symbol_table_add_func2, clone_func2,
    free_func_closure2, symbol_table_free_func2,
    a: c_double, b: c_double);
func_impl!(add_func3, symbol_table_add_func3, clone_func3,
    free_func_closure3, symbol_table_free_func3,
    a: c_double, b: c_double, c: c_double);
func_impl!(add_func4, symbol_table_add_func4, clone_func4,
    free_func_closure4, symbol_table_free_func4,
    a: c_double, b: c_double, c: c_double, d: c_double);
func_impl!(add_func5, symbol_table_add_func5, clone_func5,
    free_func_closure5, symbol_table_free_func5,
    a: c_double, b: c_double, c: c_double, d: c_double, e: c_double
);
func_impl!(add_func6, symbol_table_add_func6, clone_func6,
    free_func_closure6, symbol_table_free_func6,
    a: c_double, b: c_double, c: c_double, d: c_double, e: c_double, f: c_double
);
func_impl!(add_func7, symbol_table_add_func7, clone_func7,
    free_func_closure7, symbol_table_free_func7,
    a: c_double, b: c_double, c: c_double, d: c_double, e: c_double, f: c_double, g: c_double
);
func_impl!(add_func8, symbol_table_add_func8, clone_func8,
    free_func_closure8, symbol_table_free_func8,
    a: c_double, b: c_double, c: c_double, d: c_double, e: c_double, f: c_double, g: c_double,
    h: c_double
);
func_impl!(add_func9, symbol_table_add_func9, clone_func9,
    free_func_closure9, symbol_table_free_func9,
    a: c_double, b: c_double, c: c_double, d: c_double, e: c_double, f: c_double, g: c_double,
    h: c_double, i: c_double
);
func_impl!(add_func10, symbol_table_add_func10, clone_func10,
    free_func_closure10, symbol_table_free_func10,
    a: c_double, b: c_double, c: c_double, d: c_double, e: c_double, f: c_double, g: c_double,
    h: c_double, i: c_double, j: c_double
);

impl Default for exprtk::SymbolTable {
     fn default() -> Self {
         Self::new()
     }
}

impl Drop for SymbolTable {
    fn drop(&mut self) {
        // strings have their owne destructor, but function pointers need to be freed
        for f in &self.funcs {
            unsafe {
                (f.free_cpp_func)(f.cpp_func);
                (f.free_closure_func)(f.rust_closure);
            }
        }
        unsafe { symbol_table_destroy(self.sym) };
    }
}


impl fmt::Debug for SymbolTable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let varnames = self.get_variable_names();
        write!(f,
            "SymbolTable {{ values: {}, constants: {}, strings: {}, vectors: {:?}, functions: {} }}",
            format!("[{}]", varnames
                .iter()
                .filter(|n| !self.is_constant_node(n).unwrap())
                .map(|n| format!("\"{}\": {}", n, self.value_from_name(n).unwrap()))
                .collect::<Vec<_>>()
                .join(", ")
            ),
            format!("[{}]", varnames
                .iter()
                .filter(|n| self.is_constant_node(n).unwrap())
                .map(|n| format!("\"{}\": {}", n, self.value_from_name(n).unwrap()))
                .collect::<Vec<_>>()
                .join(", ")
            ),
            format!("[{}]", self.get_stringvar_names()
                .iter()
                .map(|n| format!("\"{}\": \"{}\"", n,
                    self.string(self.get_string_id(n).unwrap().unwrap()).unwrap().get())
                )
                .collect::<Vec<_>>()
                .join(", ")
            ),
            format!("[{}]", self.get_vector_names()
                .iter()
                .map(|n| format!("\"{}\": {:?}", n, self.vector(self.get_vec_id(n).unwrap().unwrap()).unwrap()))
                .collect::<Vec<_>>()
                .join(", ")
            ),
            format!("[{}]", self.funcs
                .iter()
                .map(|f| f.name.to_string())
                .collect::<Vec<_>>()
                .join(", ")
            ),
        )
    }
}


impl Clone for SymbolTable {
    fn clone(&self) -> SymbolTable {
        let mut s = Self::new();
        // vars
        for n in self.get_variable_names() {
            let v = self.value_from_name(&n).unwrap();
            if self.is_constant_node(&n).unwrap() {
                s.add_constant(&n, v).unwrap();
            } else {
                s.add_variable(&n, v).unwrap();
            }
        }
        // strings
        for n in self.get_stringvar_names() {
            let v = self.string(self.get_string_id(&n).unwrap().unwrap()).unwrap().get();
            s.add_stringvar(&n, &v).unwrap();
        }
        // vectors
        for n in self.get_vector_names() {
            let v = self.vector(self.get_vec_id(&n).unwrap().unwrap()).unwrap();
            s.add_vector(&n, v).unwrap();
        }
        // functions
        for f in &self.funcs {
            (f.clone_func)(&f.name, f.rust_closure, &mut s).unwrap();
        }
        s
    }
}


/// Wraps a string value and allows modifying it.
pub struct StringValue(*mut CppString);

impl StringValue {
    pub fn new(value: &str) -> StringValue {
        let s = unsafe {
                cpp_string_create(value.as_ptr() as *const c_char, value.len() as size_t)
            };
        StringValue(s)
    }

    /// Assigns a new value to the string.
    /// *Note* that setting non-ASCII values will not necessarily fail, but may result in
    /// wrong results. The length of a string (`'string[]'`) will not be correct if containing
    /// multi-byte UTF-8 characters.
    pub fn set(&mut self, value: &str) {
        unsafe {
            cpp_string_set(
                self.0,
                value.as_ptr() as *const c_char,
                value.len() as size_t,
            )
        }
    }

    /// Returns a copy of the internal string.
    pub fn get(&self) -> &str {
        unsafe { CStr::from_ptr(cpp_string_get(self.0)) }.to_str().unwrap()
    }
}

impl Drop for StringValue {
    fn drop(&mut self) {
        unsafe { cpp_string_free(self.0) };
    }
}

impl fmt::Debug for StringValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "StringValue {{ {} }}", self.get())
    }
}
