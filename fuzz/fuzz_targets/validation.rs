
use exprtk_rs::*;
use approx::relative_eq;
use arbitrary::Arbitrary;


#[derive(Debug, Arbitrary)]
pub struct Data {
    var_sym: char,
    const_sym: char,
    str_sym: char,
    vec_sym: char,
    func_sym: char,
    formula: String,
}


pub fn validate_input(data: Data) {

    let mut symbols = SymbolTable::new();

    let var_sym = data.var_sym.to_string();
    let const_sym = data.const_sym.to_string();
    let str_sym = data.str_sym.to_string();
    let vec_sym = data.vec_sym.to_string();
    let func_sym = data.func_sym.to_string();

    for s in &[data.var_sym, data.const_sym, data.str_sym, data.vec_sym, data.func_sym] {
        if *s as u8 == 0 {
            return;
        }
    }

    if let Ok(Some(var_id)) = symbols.add_variable(&var_sym, 1.) {
        symbols.value_cell(var_id).set(2.);
        symbols.value_cell(var_id);
        assert_eq!(&symbols.get_variable_names(), &[var_sym.as_str()]);
        assert!(symbols.symbol_exists(&var_sym).unwrap());
    }

    if let Ok(ok) = symbols.add_constant(&const_sym, 1.) {
        assert!(symbols.is_constant_node(&const_sym).unwrap() || const_sym == var_sym || !ok);
    }

    symbols.add_constants();
    symbols.add_pi();
    symbols.add_epsilon();
    symbols.add_infinity();

    if let Ok(Some(str_id)) = symbols.add_stringvar(&str_sym, "value") {
        assert_eq!(symbols.get_string_id(&str_sym).unwrap(), Some(str_id));
        symbols.set_string(str_id, "new value");
        symbols.string(str_id);
        assert_eq!(&symbols.get_stringvar_names(), &[str_sym.as_str()]);
        assert!(symbols.symbol_exists(&str_sym).unwrap());
        assert!(!symbols.is_constant_string(&str_sym).unwrap());
    }

    if let Ok(Some(vec_id)) = symbols.add_vector(&vec_sym, &[1., 2.]) {
        assert_eq!(symbols.get_vec_id(&vec_sym).unwrap(), Some(vec_id));
        symbols.vector_mut(vec_id).unwrap()[0] = 0.;
        assert_eq!(&symbols.get_vector_names(), &[vec_sym.as_str()]);
        assert!(symbols.symbol_exists(&vec_sym).unwrap());
    }

    symbols.add_func1(&func_sym, |x| x).ok();

    let mut symbols2 = symbols.clone();
    Expression::new(&data.formula, symbols.clone()).ok();

    assert_eq!(format!("{:?}", symbols), format!("{:?}", symbols2));

    if let Ok((mut expr, vars)) = Expression::parse_vars(&data.formula, symbols.clone()) {
        // add these vars to original symbol table
        for (v, _) in vars {
            symbols.add_variable(&v, 0.).unwrap();
        }
        let mut expr2 = Expression::new(&data.formula, symbols).unwrap();
        let mut expr3 = expr.clone();
        let mut expr4 = expr2.clone();
        assert_eq!(format!("{:?}", expr), format!("{:?}", expr2));
        assert_eq!(format!("{:?}", expr), format!("{:?}", expr3));
        assert_eq!(format!("{:?}", expr), format!("{:?}", expr4));

        // Compare values. Has to be done AFTER cloning of expr, since a call to
        // Expression::value() can modify its internal SymbolTable).
        let v = expr.value();
        let v2 = expr2.value();
        let v3 = expr3.value();
        let v4 = expr4.value();
        assert!(relative_eq!(v, v2) || v.is_nan() && v2.is_nan());
        assert!(relative_eq!(v, v3) || v.is_nan() && v3.is_nan());
        assert!(relative_eq!(v, v4) || v.is_nan() && v4.is_nan());
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
