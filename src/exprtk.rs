use std::ops::Drop;
use std::ffi::*;
use std::ptr;
use std::mem::transmute;
use std::fmt;
use std::cell::Cell;
use enum_primitive::FromPrimitive;

use libc::{c_char, size_t, c_double, c_void};
use exprtk_sys::*;
use super::*;


macro_rules! c_string {
    ($s:expr) => { CString::new($s).expect("String contains nul byte.").as_ptr() }
}

macro_rules! string_from_ptr {
    ($s:expr) => { CStr::from_ptr($s).to_str().unwrap().to_string() }
}


unsafe impl Send for Parser {}
unsafe impl Send for Expression {}
unsafe impl Send for SymbolTable {}


#[derive(Debug)]
struct Parser(*mut CParser);

impl Parser {
    pub fn new() -> Parser {
        unsafe { Parser(parser_new()) }
    }

    pub fn compile(&self, string: &str, expr: &Expression) -> Result<(), ParseError> {
        unsafe {
            if !parser_compile(self.0, c_string!(string), expr.expr) {
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
        let expr_ptr = expr.expr;
        let symbols = expr.symbols_mut();
        let mut user_data = (symbols, &mut func);
        unsafe {
            let r = parser_compile_resolve(self.0, c_string!(string), expr_ptr, wrapper::<F, S>, &mut user_data as *const _ as *mut c_void);
            if !r {
                return Err(self.get_err());
            }
        };

        extern fn wrapper<F, S>(c_name: *const c_char, user_data: *mut c_void) -> *const c_char
        where F: FnMut(&str, &mut SymbolTable) -> Result<(), S>,
        S: AsRef<str>
        {
            let (ref mut symbols, ref mut opt_f): &mut (&mut SymbolTable, Option<&mut F>) = unsafe { transmute(user_data) };
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

    unsafe fn get_err(&self) -> ParseError {
        let e: &mut CParseError = transmute(parser_error(self.0));
        if e.is_err {
            let err_out = ParseError {
                kind: ParseErrorKind::from_i32(e.mode as i32).expect(&format!(
                    "Unknown ParseErrorKind enum variant: {}",
                    e.mode
                )),
                token_type: string_from_ptr!(e.token_type),
                token_value: string_from_ptr!(e.token_value),
                message: string_from_ptr!(e.diagnostic),
                line: string_from_ptr!(e.error_line),
                line_no: e.line_no as usize,
                column_no: e.column_no as usize,
            };
            parser_error_free(e as *mut CParseError);
            err_out
        } else {
            panic!("Compiler notified about error, but there is none.")
        }
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
    /// let expr = Expression::new("a + 1", symbol_table).unwrap();
    /// assert_eq!(expr.value(), 3.);
    /// ```
    pub fn new(string: &str, symbols: SymbolTable) -> Result<Expression, ParseError> {
        let parser = Parser::new();
        let e = Expression {
            expr: unsafe { expression_new() },
            string: string.to_string(),
            symbols: symbols,
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
    ///         sym.add_stringvar(name, b"string").unwrap();
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
            symbols: symbols,
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
    pub fn value(&self) -> c_double {
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
    clone_func: fn(&str, *mut c_void, &mut SymbolTable) -> Result<bool, InvalidName>
}

/// `SymbolTable` holds different variables. There are three types of variables:
/// Numberic variables, strings and numeric vectors of fixed size. (see
/// [the documentation](https://github.com/ArashPartow/exprtk/blob/f32d2b4bbb640ea4732b8a7fce1bd9717e9c998b/readme.txt#L643)).
/// Many but not all of the methods of the [ExprTk symbol_table](http://partow.net/programming/exprtk/doxygen/classexprtk_1_1symbol__table.html)
/// were implemented, and the API is sometimes different.
pub struct SymbolTable {
    sym: *mut CSymbolTable,
    values: Vec<*mut c_double>,
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
        let rv = unsafe { symbol_table_add_constant(self.sym, c_string!(name), value) };
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
        // TODO: examine whether it is slower to store the values themselves in the Rust struct
        // (as Cells) and supply the pointers to ExprTk using symbol_table_add_variable
        // would be easier
        let rv =
            unsafe { symbol_table_create_variable(self.sym, c_string!(name), value as c_double) };
        let res = self.validate_added(name, rv, i)?;
        let ptr = unsafe { symbol_table_variable_ref(self.sym, c_string!(name)) };
        self.values.push(ptr);
        Ok(res)
    }

    /// Returns the value of a registered variable as modifiable `std::cell::Cell`.
    /// This allows changing the values easily. If the reference to the `Cell` is
    /// kept around, its value can easily be changed multiple times.
    ///
    /// # Example:
    /// ```
    /// use exprtk_rs::*;
    ///
    /// let mut symbol_table = SymbolTable::new();
    /// let id = symbol_table.add_variable("a", 2.).unwrap().unwrap();
    /// let mut expr = Expression::new("a - 1", symbol_table).unwrap();
    /// assert_eq!(expr.value(), 1.);
    ///
    /// let value = expr.symbols().value(id).unwrap();
    /// value.set(4.);
    /// assert_eq!(expr.value(), 3.);
    /// ```
    #[inline]
    pub fn value(&self, var_id: usize) -> Option<&Cell<c_double>> {
        let ptr = self.values.get(var_id);
        // turn into Option<&mut c_double>
        let var_ref = unsafe { ptr.map(|p| p.as_mut().expect("null pointer!")) };
        var_ref.map(|r| {
            // TODO: workaround until Cell::from_mut() becomes stable
            // (currently behind as_cell feature flag)
            unsafe { &*(r as *mut _ as *const Cell<_>) }
        })
    }

    /// Returns the value of a variable (whether constant or not)
    #[inline]
    pub fn value_from_name(&self, name: &str) -> Option<c_double> {
        unsafe {
            symbol_table_variable_ref(self.sym, c_string!(name))
                .as_ref()
                .map(|v| *v)
        }
    }

    /// Adds a new string variable. Returns the variable ID that can later be used for `set_string`
    /// or `None` if a variable with the same name was already present.
    pub fn add_stringvar(&mut self, name: &str, text: &[u8]) -> Result<Option<usize>, InvalidName> {
        let i = self.strings.len();
        let s = StringValue::new(text);
        self.strings.push(s);
        let rv = unsafe {
            symbol_table_add_stringvar(self.sym, c_string!(name), self.strings[i].0, false)
        };
        let res = self.validate_added(name, rv, i);
        if res.is_err() {
            self.strings.pop();
        }
        res
    }

    #[inline]
    pub fn set_string(&mut self, var_id: usize, text: &[u8]) -> bool {
        if let Some(s) = self.mut_string(var_id) {
            s.set(text);
            return true;
        }
        false
    }

    #[inline]
    pub fn string(&self, var_id: usize) -> Option<&StringValue> {
        self.strings.get(var_id)
    }

    #[inline]
    pub fn mut_string(&mut self, var_id: usize) -> Option<&mut StringValue> {
        self.strings.get_mut(var_id)
    }

    /// Adds a new vector variable. Returns the variable ID that can later be used for `vector`
    /// or `None` if a variable with the same name was already present.
    pub fn add_vector(&mut self, name: &str, vec: &[c_double]) -> Result<Option<usize>, InvalidName> {
        let i = self.vectors.len();
        let l = vec.len();
        self.vectors.push(vec.to_vec().into_boxed_slice());
        let rv = unsafe {
            symbol_table_add_vector(self.sym, c_string!(name), self.vectors[i].as_ptr(), l)
        };
        let res = self.validate_added(name, rv, i);
        if res.is_err() {
            self.vectors.pop();
        }
        res
    }

    #[inline]
    pub fn vector(&self, var_id: usize) -> Option<&[c_double]> {
        self.vectors.get(var_id).map(|v| &**v)
    }

    #[inline]
    pub fn mut_vector(&mut self, var_id: usize) -> Option<&mut [c_double]> {
        self.vectors.get_mut(var_id).map(|v| &mut **v)
    }

    fn validate_added<O>(&self, name: &str, result: bool, out: O) -> Result<Option<O>, InvalidName> {
        if !result {
            let valid = unsafe { symbol_table_valid(self.sym) };
            if !valid {
                panic!("Bug: SymbolTable state invalid!");
            }
            return if self.symbol_exists(name) {
                Ok(None)
            } else {
                Err(InvalidName(name.to_string()))
            };
        }
        Ok(Some(out))
    }

    fn get_var_ptr(&self, name: &str) -> Option<*mut c_double> {
        let rv = unsafe { symbol_table_variable_ref(self.sym, c_string!(name)) };
        if rv.is_null() { None } else { Some(rv) }
    }

    /// Returns the 'ID' of a variable or None if not found
    pub fn get_var_id(&self, name: &str) -> Option<usize> {
        self.get_var_ptr(name).and_then(|rv| {
            self.values.iter().position(|&v| v == rv)
        })
    }

    /// Returns the 'ID' of a string or None if not found
    pub fn get_string_id(&self, name: &str) -> Option<usize> {
        let ptr = unsafe { symbol_table_stringvar_ref(self.sym, c_string!(name)) };
        if ptr.is_null() {
            None
        } else {
            self.strings.iter().position(|s| s.0 == ptr)
        }
    }

    /// Returns the 'ID' of a vector or None if not found
    pub fn get_vec_id(&self, name: &str) -> Option<usize> {
        let ptr = unsafe { symbol_table_vector_ptr(self.sym, c_string!(name)) };
        if ptr.is_null() {
            None
        } else {
            self.vectors.iter().position(|v| v.as_ptr() == ptr)
        }
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

    pub fn symbol_exists(&self, name: &str) -> bool {
        unsafe { symbol_table_symbol_exists(self.sym, c_string!(name)) }
    }

    pub fn is_constant_node(&self, name: &str) -> bool {
        unsafe { symbol_table_is_constant_node(self.sym, c_string!(name)) }
    }

    pub fn is_constant_string(&self, name: &str) -> bool {
        unsafe { symbol_table_is_constant_string(self.sym, c_string!(name)) }
    }
}

macro_rules! func_impl {
    ($name:ident, $sys_func:ident, $clone_func:ident, $($x:ident: $ty:ty),*) => {
        impl SymbolTable {
            /// Add a function. Returns `true` if the function was added / `false`
            /// if the name was already present.
            pub fn $name<F>(&mut self, name: &str, func: F) -> Result<bool, InvalidName>
                where F: Fn($($ty),*) -> c_double
            {
                extern fn wrapper<F>(closure: *mut c_void, $($x: $ty),*) -> c_double
                    where F: Fn($($ty),*) -> c_double {
                    unsafe {
                        let opt_closure: Option<Box<F>> = transmute(closure);
                        opt_closure.map(|f| f($($x),*)).unwrap()
                    }
                }

                let func_box = Box::new(func);
                let func_ptr = Box::into_raw(func_box) as *mut _ as *mut c_void;
                let result = unsafe {
                    $sys_func(self.sym, c_string!(name), wrapper::<F>, func_ptr)
                };

                let res = self.validate_added(name, result.0, ())?;
                if res.is_none() {
                    self.funcs.push(FuncData {
                        name: name.to_string(),
                        cpp_func: result.1,
                        rust_closure: func_ptr,
                        clone_func: $clone_func::<F>
                    });
                }
                Ok(res.is_some())
            }
        }

        fn $clone_func<F>(name: &str, closure_ptr: *mut c_void, new_symbols: &mut SymbolTable)
        -> Result<bool, InvalidName>
        where F: Fn($($ty),*) -> c_double
        {
            let opt_closure: Option<Box<F>> = unsafe { transmute(closure_ptr) };
            new_symbols.$name(name, *opt_closure.unwrap())
        }
    }
}

func_impl!(add_func1, symbol_table_add_func1, clone_func1, a: c_double);
func_impl!(add_func2, symbol_table_add_func2, clone_func2, a: c_double, b: c_double);
func_impl!(add_func3, symbol_table_add_func3, clone_func3, a: c_double, b: c_double, c: c_double);
func_impl!(add_func4, symbol_table_add_func4, clone_func4, a: c_double, b: c_double, c: c_double,
    d: c_double);
func_impl!(add_func5, symbol_table_add_func5, clone_func5, a: c_double, b: c_double, c: c_double,
    d: c_double, e: c_double
);
func_impl!(add_func6, symbol_table_add_func6, clone_func6, a: c_double, b: c_double, c: c_double,
    d: c_double, e: c_double, f: c_double
);
func_impl!(add_func7, symbol_table_add_func7, clone_func7, a: c_double, b: c_double, c: c_double,
    d: c_double, e: c_double, f: c_double, g: c_double
);
func_impl!(add_func8, symbol_table_add_func8, clone_func8, a: c_double, b: c_double, c: c_double,
    d: c_double, e: c_double, f: c_double, g: c_double, h: c_double
);
func_impl!(add_func9, symbol_table_add_func9, clone_func9, a: c_double, b: c_double, c: c_double,
    d: c_double, e: c_double, f: c_double, g: c_double, h: c_double, i: c_double
);
func_impl!(add_func10, symbol_table_add_func10, clone_func10, a: c_double, b: c_double, c: c_double,
    d: c_double, e: c_double, f: c_double, g: c_double, h: c_double, i: c_double, j: c_double
);


impl Drop for SymbolTable {
    fn drop(&mut self) {
        // strings have their owne destructor, but function pointers need to be freed
        for f in &self.funcs {
            unsafe { symbol_table_free_func1(f.cpp_func) };
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
                .filter(|n| !self.is_constant_node(n))
                .map(|n| format!("\"{}\": {}", n, self.value_from_name(n).unwrap()))
                .collect::<Vec<_>>()
                .join(",")
            ),
            format!("[{}]", varnames
                .iter()
                .filter(|n| self.is_constant_node(n))
                .map(|n| format!("\"{}\": {}", n, self.value_from_name(n).unwrap()))
                .collect::<Vec<_>>()
                .join(",")
            ),
            format!("[{}]", self.get_stringvar_names()
                .iter()
                .map(|n| format!("\"{}\": \"{}\"", n, String::from_utf8_lossy(
                    self.string(self.get_string_id(n).unwrap()).unwrap().get())
                ))
                .collect::<Vec<_>>()
                .join(",")
            ),
            format!("[{}]", self.get_vector_names()
                .iter()
                .map(|n| format!("\"{}\": {:?}", n, self.vector(self.get_vec_id(n).unwrap()).unwrap()))
                .collect::<Vec<_>>()
                .join(",")
            ),
            format!("[{}]", self.funcs
                .iter()
                .map(|f| format!("{}", f.name))
                .collect::<Vec<_>>()
                .join(",")
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
            if self.is_constant_node(&n) {
                s.add_constant(&n, v).unwrap();
            } else {
                s.add_variable(&n, v).unwrap();
            }
        }
        // strings
        for n in self.get_stringvar_names() {
            let v = self.string(self.get_string_id(&n).unwrap()).unwrap().get();
            s.add_stringvar(&n, &v).unwrap();
        }
        // vectors
        for n in self.get_vector_names() {
            let v = self.vector(self.get_vec_id(&n).unwrap()).unwrap();
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
    pub fn new(value: &[u8]) -> StringValue {
        let s = unsafe {
                cpp_string_create(value.as_ptr() as *const c_char, value.len() as size_t)
            };
        StringValue(s)
    }

    /// Assigns a new value to the string.
    pub fn set(&mut self, value: &[u8]) {
        unsafe {
            cpp_string_set(
                self.0,
                value.as_ptr() as *const c_char,
                value.len() as size_t,
            )
        }
    }

    /// Returns a copy of the internal string.
    pub fn get(&self) -> &[u8] {
        unsafe { CStr::from_ptr(cpp_string_get(self.0)) }.to_bytes()
    }
}

impl Drop for StringValue {
    fn drop(&mut self) {
        unsafe { cpp_string_free(self.0) };
    }
}

impl fmt::Debug for StringValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "StringValue {{ {} }}", String::from_utf8_lossy(self.get()))
    }
}
