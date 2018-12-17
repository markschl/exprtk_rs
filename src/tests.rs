
use super::*;

#[test]
fn test_var() {
    let mut s = SymbolTable::new();
    let a_id = s.add_variable("a", 0.).unwrap().unwrap();
    let e = Expression::new("a + a / 2", s).unwrap();
    assert_relative_eq!(e.value(), 0.);
    e.symbols().value(a_id).unwrap().set(2.);
    assert_relative_eq!(e.value(), 3.);
}

#[test]
fn test_constant() {
    let mut s = SymbolTable::new();
    s.add_constant("a", 2.).unwrap();
    let e = Expression::new("a + a / 2", s).unwrap();
    assert_relative_eq!(e.value(), 3.);
}

#[test]
fn test_string() {
    let mut s = SymbolTable::new();
    let s_id = s.add_stringvar("s", b"string").unwrap().unwrap();
    let mut e = Expression::new("s[] + 1", s).unwrap();
    assert_relative_eq!(e.value(), 7.);
    e.symbols_mut().set_string(s_id, b"string2");
    assert_relative_eq!(e.value(), 8.);
}

#[test]
fn test_vector() {
    let mut s = SymbolTable::new();
    let vec_id = s.add_vector("v", &[0., 1., 2., 3.]).unwrap().unwrap();
    let mut e = Expression::new("v[] + v[1] + 1", s).unwrap();
    assert_relative_eq!(e.value(), 6.);
    e.symbols_mut().vector(vec_id).unwrap()[1].set(2.);
    assert_relative_eq!(e.value(), 7.);
}

#[test]
fn test_vector_out_of_bounds() {
    let mut s = SymbolTable::new();
    s.add_vector("v", &[0., 1., 2., 3.]).unwrap().unwrap();
    if let Err(e) = Expression::new("v[1] + v[4]", s) {
        assert!(e.message.contains("out of range for vector"));
    } else {
        panic!("Should fail!");
    }
}

#[test]
fn test_funcs() {
    let mut s = SymbolTable::new();
    s.add_func1("add_one", |a| a + 1.).unwrap();
    let e = Expression::new("add_one(1)", s).unwrap();
    assert_relative_eq!(e.value(), 2.);

    let mut s = SymbolTable::new();
    s.add_func1("one", |a| a).unwrap();
    s.add_func2("two", |a, b| a + b).unwrap();
    s.add_func3("three", |a, b, c| a + b + c).unwrap();
    s.add_func4("four", |a, b, c, d| a + b + c + d).unwrap();
    let e = Expression::new("one(1) + two(1, 1) + three(1, 1 ,1) + four(1, 1, 1, 1)", s).unwrap();
    assert_relative_eq!(e.value(), 10.);
}

#[test]
fn test_parse_err() {
    let mut s = SymbolTable::new();
    s.add_variable("a", 1.).unwrap().unwrap();
    let expr = Expression::new("a + 1 + b", s);
    if let Err(e) = expr {
        assert_eq!(e.kind, ParseErrorKind::Syntax);
        assert_eq!(e.token_type, "SYMBOL".to_string());
        assert_eq!(e.token_value, "b".to_string());
        assert!(e.message.ends_with(" - Undefined symbol: 'b'"));
        assert_eq!(e.line, "".to_string());
        assert_eq!(e.line_no, 0);
        assert_eq!(e.column_no, 0);
    } else {
        panic!("Should fail!");
    }
}

#[test]
fn test_resolver() {
    let mut s = SymbolTable::new();
    s.add_variable("a", 1.).unwrap().unwrap();
    let expr = Expression::handle_unknown("a + b + c + s[] + v[]", s, |name, s| {
        match name {
            "b" => { s.add_variable(name, 1.).unwrap(); },
            "c" => { s.add_constant(name, 1.).unwrap(); },
            "s" => { s.add_stringvar(name, b"string").unwrap(); },
            "v" => { s.add_vector(name, &[1., 2., 3.]).unwrap(); },
            _ => {}
        }
        Ok(())
    }).unwrap();
    assert_relative_eq!(expr.value(), 12.);
}

#[test]
fn test_auto_resolver() {
    let (expr, vars) = Expression::parse_vars("a + b", SymbolTable::new()).unwrap();
    assert_eq!(vars, vec![("a".to_string(), 0), ("b".to_string(), 1)]);
    assert_relative_eq!(expr.value(), 0.);
    expr.symbols().value(0).unwrap().set(1.);
    assert_relative_eq!(expr.value(), 1.);
}

#[test]
fn test_names() {
    let mut s = SymbolTable::new();
    s.add_variable("a", 1.).unwrap().unwrap();
    s.add_stringvar("s", b"value").unwrap().unwrap();
    s.add_vector("v", &[1., 2.]).unwrap().unwrap();
    let (expr, _) = Expression::parse_vars("a + 1 + b + s[] + v[]", s).unwrap();
    assert_eq!(expr.symbols().get_variable_names(), vec!["a", "b"]);
    assert_eq!(expr.symbols().get_stringvar_names(), vec!["s"]);
    assert_eq!(expr.symbols().get_vector_names(), vec!["v"]);
}

#[test]
fn test_clear() {
    let mut s = SymbolTable::new();
    s.add_variable("a", 1.).unwrap().unwrap();
    s.add_stringvar("s", b"value").unwrap().unwrap();
    s.add_vector("v", &[1., 2.]).unwrap().unwrap();
    s.clear_variables();
    s.clear_strings();
    s.clear_vectors();
    assert!(s.get_variable_names().is_empty());
    assert!(s.get_stringvar_names().is_empty());
    assert!(s.get_vector_names().is_empty());
}

#[test]
fn test_const() {
    let mut s = SymbolTable::new();
    s.add_constant("a", 1.).unwrap();
    let expr = Expression::new("a + 1", s).unwrap();
    assert_relative_eq!(expr.value(), 2.);
}


#[test]
fn test_ids() {
    let mut s = SymbolTable::new();
    let a_id = s.add_variable("a", 1.).unwrap().unwrap();
    let b_id = s.add_variable("b", 1.).unwrap().unwrap();
    let s_id = s.add_stringvar("s", b"value").unwrap().unwrap();
    let v_id = s.add_vector("v", &[1., 2.]).unwrap().unwrap();
    assert_eq!(s.get_var_id("a"), Some(a_id));
    assert_eq!(s.get_var_id("b"), Some(b_id));
    assert_eq!(s.get_var_id("c"), None);
    assert_eq!(s.get_string_id("s"), Some(s_id));
    assert_eq!(s.get_vec_id("v"), Some(v_id));
    assert_eq!(s.get_vec_id("s"), None);
}

#[test]
fn test_clone() {
    let mut s = SymbolTable::new();
    s.add_variable("a", 1.).unwrap().unwrap();
    s.add_stringvar("s", b"s").unwrap().unwrap();
    s.add_vector("v", &[1., 2.]).unwrap().unwrap();
    s.add_func1("func", |x| x + 1.).unwrap();
    let expr = Expression::new("a + s[] + v[0] + func(0)", s).unwrap();
    assert_relative_eq!(expr.value(), 4.);
    assert_relative_eq!(expr.clone().value(), 4.);
    assert_eq!(format!("{:?}", expr), format!("{:?}", expr.clone()));
}

#[test]
fn test_send() {
    use std::thread;
    let mut s = SymbolTable::new();
    let a_id = s.add_variable("a", 1.).unwrap().unwrap();
    let s_id = s.add_stringvar("s", b"s").unwrap().unwrap();
    let v_id = s.add_vector("v", &[1.]).unwrap().unwrap();
    let mut e = Expression::new("a + s[] + v[0]", s).unwrap();
    assert_relative_eq!(e.value(), 3.);

    thread::spawn(move || {
        assert_relative_eq!(e.value(), 3.);
        e.symbols().value(a_id).unwrap().set(2.);
        e.symbols_mut().set_string(s_id, b"s2");
        e.symbols_mut().vector(v_id).unwrap()[0].set(2.);
        assert_relative_eq!(e.value(), 6.);
    });
}
