#![no_main]
#[macro_use] extern crate libfuzzer_sys;
extern crate exprtk_rs;

use exprtk_rs::*;

fuzz_target!(|data: &[u8]| {
    if let Ok(formula) = ::std::str::from_utf8(data) {
        if data.len() <= 5 || !formula.is_ascii() || formula.as_bytes().into_iter().any(|&b| b == 0) {
            return;
        }

        let mut c = formula.chars();
        let var_sym = c.next().unwrap().to_string();
        let const_sym = c.next().unwrap().to_string();
        let str_sym = c.next().unwrap().to_string();
        let vec_sym = c.next().unwrap().to_string();
        let func_sym = c.next().unwrap().to_string();

        let formula: String = c.collect();

        let mut symbols = SymbolTable::new();

        if let Ok(Some(var_id)) = symbols.add_variable(&var_sym, 1.) {
            symbols.value(var_id).unwrap().set(2.);
            symbols.value(var_id);
            assert_eq!(&symbols.get_variable_names(), &[var_sym.as_str()]);
            assert!(symbols.symbol_exists(&var_sym));
        }

        if let Ok(ok) = symbols.add_constant(&const_sym, 1.) {
            assert!(symbols.is_constant_node(&const_sym) || const_sym == var_sym || !ok);
        }

        symbols.add_constants();
        symbols.add_pi();
        symbols.add_epsilon();
        symbols.add_infinity();

        if let Ok(Some(str_id)) = symbols.add_stringvar(&str_sym, b"value") {
            assert_eq!(symbols.get_string_id(&str_sym), Some(str_id));
            symbols.set_string(str_id, b"new value");
            symbols.string(str_id);
            assert_eq!(&symbols.get_stringvar_names(), &[str_sym.as_str()]);
            assert!(symbols.symbol_exists(&str_sym));
            assert!(!symbols.is_constant_string(&str_sym));
        }

        if let Ok(Some(vec_id)) = symbols.add_vector(&vec_sym, &[1., 2.]) {
            assert_eq!(symbols.get_vec_id(&vec_sym), Some(vec_id));
            symbols.vector(vec_id).unwrap()[0].set(0.);
            assert_eq!(&symbols.get_vector_names(), &[vec_sym.as_str()]);
            assert!(symbols.symbol_exists(&vec_sym));
        }

        symbols.add_func1(&func_sym, |x| x).ok();

        let mut symbols2 = symbols.clone();
        Expression::new(&formula, symbols.clone()).ok();

        assert_eq!(format!("{:?}", symbols), format!("{:?}", symbols2));

        if let Ok((expr, vars)) = Expression::parse_vars(&formula, symbols.clone()) {
            let v = expr.value();
            // add these vars to original symbol table
            for (v, _) in vars {
                symbols.add_variable(&v, 0.).unwrap();
            }
            let v2 = Expression::new(&formula, symbols).unwrap().value();
            assert!(v == v2 || v.is_nan() && v2.is_nan());
            let expr2 = expr.clone();
            let v3 = expr2.value();
            assert!(v == v3 || v.is_nan() && v3.is_nan());
            assert_eq!(format!("{:?}", expr), format!("{:?}", expr2));
        }

        symbols2.clear_variables();
        symbols2.clear_strings();
        symbols2.clear_vectors();
        symbols2.clear_local_constants();
        symbols2.clear_functions();
        assert_eq!(symbols2.variable_count(), 0);
        assert_eq!(symbols2.stringvar_count(), 0);
        assert_eq!(symbols2.vector_count(), 0);
        assert_eq!(symbols2.function_count(), 0);
    }
});
