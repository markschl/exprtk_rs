
extern crate libc;

use std::slice;
use libc::*;
use std::ffi::CString;

// types

pub enum CSymbolTable {}
pub enum CExpression {}
pub enum CParser {}
pub enum CppString {}


// simple types used for communications with C++

#[repr(C)]
pub struct Pair<T, U>(pub T, pub U);

pub type CStrList = Pair<size_t, *const *const c_char>;

impl CStrList {
    pub unsafe fn get_slice<'a>(&'a self) -> &'a [*const c_char] {
        slice::from_raw_parts(self.1, self.0 as usize)
    }
}

#[repr(C)]
pub struct CParseError {
    pub is_err: bool,
    pub mode: c_int,
    pub token_type: *const c_char,
    pub token_value: *const c_char,
    pub diagnostic: *const c_char,
    pub error_line: *const c_char,
    pub line_no: size_t,
    pub column_no: size_t,
}


// for deallocating CString from C
#[no_mangle]
pub extern fn free_rust_cstring(s: *mut c_char) {
    let _ = unsafe { CString::from_raw(s) };
}


// functions without polymorphism
extern "C" {

    // these methods depend on a specific precision

    pub fn symbol_table_new() -> *mut CSymbolTable;
    pub fn symbol_table_add_variable(t: *mut CSymbolTable,
        variable_name: *const c_char, value: *const c_double, is_constant: bool) -> bool;
    pub fn symbol_table_add_constant(t: *mut CSymbolTable,
        variable_name: *const c_char, value: c_double) -> bool;
    pub fn symbol_table_create_variable(t: *mut CSymbolTable,
         variable_name: *const c_char, value: c_double) -> bool;
    pub fn symbol_table_add_stringvar(t: *mut CSymbolTable,
        variable_name: *const c_char, string: *mut CppString, is_const: bool) -> bool;
    pub fn symbol_table_create_stringvar(t: *mut CSymbolTable,
        variable_name: *const c_char, string: *const c_char) -> bool;
    pub fn symbol_table_add_vector(t: *mut CSymbolTable,
        variable_name: *const c_char, ptr: *const c_double, len: size_t) -> bool;
    pub fn symbol_table_remove_variable(t: *mut CSymbolTable, name: *const c_char) -> bool;
    pub fn symbol_table_remove_stringvar(t: *mut CSymbolTable, name: *const c_char) -> bool;
    pub fn symbol_table_remove_vector(t: *mut CSymbolTable, name: *const c_char) -> bool;
    pub fn symbol_table_clear_variables(t: *mut CSymbolTable);
    pub fn symbol_table_clear_strings(t: *mut CSymbolTable);
    pub fn symbol_table_clear_vectors(t: *mut CSymbolTable);
    pub fn symbol_table_clear_local_constants(t: *mut CSymbolTable);
    pub fn symbol_table_clear_functions(t: *mut CSymbolTable);
    pub fn symbol_table_variable_ref(t: *mut CSymbolTable, variable_name: *const c_char) -> *mut c_double;
    pub fn symbol_table_stringvar_ref(t: *mut CSymbolTable, variable_name: *const c_char) -> *mut CppString;
    pub fn symbol_table_vector_ptr(t: *mut CSymbolTable, variable_name: *const c_char) -> *const c_double;
    pub fn symbol_table_set_string(t: *mut CSymbolTable, ptr: *const CppString, string: *const c_char);
    pub fn symbol_table_variable_count(t: *mut CSymbolTable) -> size_t;
    pub fn symbol_table_stringvar_count(t: *mut CSymbolTable) -> size_t;
    pub fn symbol_table_vector_count(t: *mut CSymbolTable) -> size_t;
    pub fn symbol_table_function_count(t: *mut CSymbolTable) -> size_t;
    pub fn symbol_table_add_pi(t: *mut CSymbolTable) -> bool;
    pub fn symbol_table_add_epsilon(t: *mut CSymbolTable) -> bool;
    pub fn symbol_table_add_infinity(t: *mut CSymbolTable) -> bool;
    pub fn symbol_table_get_variable_list(t: *mut CSymbolTable) -> *mut CStrList;
    pub fn symbol_table_get_stringvar_list(t: *mut CSymbolTable) -> *mut CStrList; //StringPtrList;
    pub fn symbol_table_get_vector_list(t: *mut CSymbolTable) -> *mut CStrList;
    pub fn symbol_table_valid(t: *mut CSymbolTable) -> bool;
    pub fn symbol_table_symbol_exists(t: *mut CSymbolTable, name: *const c_char) -> bool;
    pub fn symbol_table_load_from(t: *mut CSymbolTable, other: *const CSymbolTable);
    pub fn symbol_table_add_constants(t: *mut CSymbolTable) -> bool;
    pub fn symbol_table_is_constant_node(t: *mut CSymbolTable, name: *const c_char) -> bool;
    pub fn symbol_table_is_constant_string(t: *mut CSymbolTable, name: *const c_char) -> bool;
    pub fn symbol_table_destroy(t: *mut CSymbolTable);

    // // blocked by #5668
    // macro_rules! func_declare {
    //     ($add_name:ident, $free_name:ident, $($ty:ty),*) => {
    //         pub fn $add_name(t: *mut CSymbolTable, name: *const c_char,
    //             cb: extern fn (*mut c_void, $($ty),*) -> c_double,
    //             user_data: *mut c_void) -> Pair<bool, *mut c_void>;
    //         pub fn $free_name(c_func: *mut c_void);
    //     }
    // }

    pub fn symbol_table_add_func1(t: *mut CSymbolTable, name: *const c_char,
        cb: extern fn (*mut c_void, c_double) -> c_double,
        user_data: *mut c_void) -> Pair<bool, *mut c_void>;
    pub fn symbol_table_free_func1(c_func: *mut c_void);

    pub fn symbol_table_add_func2(t: *mut CSymbolTable, name: *const c_char,
        cb: extern fn (*mut c_void, c_double, c_double) -> c_double,
        user_data: *mut c_void) -> Pair<bool, *mut c_void>;
    pub fn symbol_table_free_func2(c_func: *mut c_void);

    pub fn symbol_table_add_func3(t: *mut CSymbolTable, name: *const c_char,
        cb: extern fn (*mut c_void, c_double, c_double, c_double) -> c_double,
        user_data: *mut c_void) -> Pair<bool, *mut c_void>;
    pub fn symbol_table_free_func3(c_func: *mut c_void);

    pub fn symbol_table_add_func4(t: *mut CSymbolTable, name: *const c_char,
        cb: extern fn (*mut c_void, c_double, c_double, c_double, c_double) -> c_double,
        user_data: *mut c_void) -> Pair<bool, *mut c_void>;
    pub fn symbol_table_free_func4(c_func: *mut c_void);

    pub fn symbol_table_add_func5(t: *mut CSymbolTable, name: *const c_char,
        cb: extern fn (*mut c_void, c_double, c_double, c_double, c_double, c_double) -> c_double,
        user_data: *mut c_void) -> Pair<bool, *mut c_void>;
    pub fn symbol_table_free_func5(c_func: *mut c_void);

    pub fn symbol_table_add_func6(t: *mut CSymbolTable, name: *const c_char,
        cb: extern fn (*mut c_void, c_double, c_double, c_double, c_double, c_double,
        c_double) -> c_double,
        user_data: *mut c_void) -> Pair<bool, *mut c_void>;
    pub fn symbol_table_free_func6(c_func: *mut c_void);

    pub fn symbol_table_add_func7(t: *mut CSymbolTable, name: *const c_char,
        cb: extern fn (*mut c_void, c_double, c_double, c_double, c_double, c_double,
        c_double, c_double) -> c_double,
        user_data: *mut c_void) -> Pair<bool, *mut c_void>;
    pub fn symbol_table_free_func7(c_func: *mut c_void);

    pub fn symbol_table_add_func8(t: *mut CSymbolTable, name: *const c_char,
        cb: extern fn (*mut c_void, c_double, c_double, c_double, c_double, c_double,
        c_double, c_double, c_double) -> c_double,
        user_data: *mut c_void) -> Pair<bool, *mut c_void>;
    pub fn symbol_table_free_func8(c_func: *mut c_void);

    pub fn symbol_table_add_func9(t: *mut CSymbolTable, name: *const c_char,
        cb: extern fn (*mut c_void, c_double, c_double, c_double, c_double, c_double,
        c_double, c_double, c_double, c_double) -> c_double,
        user_data: *mut c_void) -> Pair<bool, *mut c_void>;
    pub fn symbol_table_free_func9(c_func: *mut c_void);

    pub fn symbol_table_add_func10(t: *mut CSymbolTable, name: *const c_char,
        cb: extern fn (*mut c_void, c_double, c_double, c_double, c_double, c_double,
        c_double, c_double, c_double, c_double, c_double) -> c_double,
        user_data: *mut c_void) -> Pair<bool, *mut c_void>;
    pub fn symbol_table_free_func10(c_func: *mut c_void);


    // Expression
    pub fn expression_new() -> *mut CExpression;
    pub fn expression_register_symbol_table(e: *mut CExpression, t: *const CSymbolTable);
    pub fn expression_value(e: *mut CExpression) -> c_double;
    pub fn expression_destroy(e: *mut CExpression);

    pub fn parser_new() -> *mut CParser;
    pub fn parser_destroy(p: *mut CParser);
    pub fn parser_compile(p: *mut CParser, s: *const c_char, e: *const CExpression) -> bool;
    pub fn parser_compile_resolve(p: *mut CParser, s: *const c_char, e: *const CExpression,
        cb: extern fn (*const c_char, *mut c_void) -> *const c_char, fn_pointer: *mut c_void) -> bool;
    pub fn parser_error(p: *mut CParser) -> *const CParseError;
    pub fn parser_error_free(p: *const CParseError);

    pub fn string_array_free(l: *mut CStrList);

    pub fn cpp_string_create(s: *const c_char, len: size_t) -> *mut CppString;
    pub fn cpp_string_set(s: *mut CppString, replacement: *const c_char, len: size_t);
    pub fn cpp_string_get(s: *const CppString) -> *const c_char;
    pub fn cpp_string_free(s: *mut CppString);

}
