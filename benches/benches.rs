#![feature(test)]
#![allow(unused_variables)]

extern crate test;
extern crate exprtk_rs;
extern crate exprtk_sys;

use std::f64::consts::PI;
use exprtk_rs::*;
use exprtk_sys::*;
use self::test::Bencher;


// These benchmarks are equivalent to some of the ExprTk benchmarks

const XMIN: c_double = -100.;
const XMAX: c_double = 100.;
const YMIN: c_double = -100.;
const YMAX: c_double = 100.;
const DELTA: c_double = 0.0111;


macro_rules! bench {
    ($name:ident, $name_id:ident, $name_noset:ident, $name_set_unsafe:ident,
        $formula:expr, $name_native:ident,
        $x:ident, $y:ident, $expr:expr) => {
        // "Normal" usage of API
        #[bench]
        fn $name(b: &mut Bencher) {
            let mut s = SymbolTable::new();
            s.add_pi();
            let x_id = s.add_variable("x", 0.).unwrap().unwrap();
            let y_id = s.add_variable("y", 0.).unwrap().unwrap();
            let e = Expression::new($formula, s).unwrap();
            let x_value = e.symbols().value(x_id).unwrap();
            let y_value = e.symbols().value(y_id).unwrap();

            b.iter(|| {
                let mut total = 0.;
                x_value.set(XMIN);
                y_value.set(YMIN);
                while x_value.get() < XMAX {
                    x_value.set(x_value.get() + DELTA);
                    while y_value.get() < YMAX {
                        y_value.set(y_value.get() + DELTA);
                        total += e.value();
                    }
                }
            });
        }

        // Retrieve the Cell each time again for changing the value
        #[bench]
        fn $name_id(b: &mut Bencher) {
            let mut s = SymbolTable::new();
            s.add_pi();
            let x_id = s.add_variable("x", 0.).unwrap().unwrap();
            let y_id = s.add_variable("y", 0.).unwrap().unwrap();
            let e = Expression::new($formula, s).unwrap();
            b.iter(|| {
                let mut total = 0.;
                let mut x = XMIN;
                let mut y = YMIN;
                while x < XMAX {
                    x += DELTA;
                    while y < YMAX {
                        y += DELTA;
                        e.symbols().value(x_id).unwrap().set(x as c_double);
                        e.symbols().value(y_id).unwrap().set(y as c_double);
                        total += e.value();
                    }
                }
            });
        }

        // Simulating the behaviour in C++ (as good as possible):
        // The pointers are directly incremented
        #[bench]
        fn $name_set_unsafe(b: &mut Bencher) {

            macro_rules! c_string {
                ($s:expr) => { ::std::ffi::CString::new($s).unwrap().as_ptr() }
            }

            let mut x = 0.;
            let mut y = 0.;

            unsafe {
                let s = symbol_table_new();
                symbol_table_add_pi(s);
                symbol_table_add_variable(s, c_string!("x"), &x as *const _, false);
                symbol_table_add_variable(s, c_string!("y"), &y as *const _, false);

                let e = expression_new();
                expression_register_symbol_table(e, s);

                let p = parser_new();
                parser_compile(p, c_string!($formula), e);

                b.iter(|| {
                    let mut total = 0.;
                    x = XMIN;
                    y = YMIN;
                    while x < XMAX {
                        x += DELTA;
                        while y < YMAX  {
                            y += DELTA;
                            total += expression_value(e);
                        }
                    }
                });
                parser_destroy(p);
                symbol_table_destroy(s);
                expression_destroy(e);
            }
        }

        // Native representation of the same formula
        #[bench]
        fn $name_native(b: &mut Bencher) {
            b.iter(|| {
                let mut total = 0.;
                let mut $x = XMIN;
                let mut $y = YMIN;
                while $x < XMAX {
                    $x += DELTA;
                    while $y < YMAX {
                        $y += DELTA;
                        total += test::black_box($expr);
                    }
                }
            });
        }
     };
}



bench!(bench1, bench1_id, bench1_noset, bench1_unsafe,
    "(y + x)",
    bench1_native, x, y, x + y
);

bench!(bench2, bench2_id, bench2_noset, bench2_unsafe,
    "2 * (y + x)",
    bench2_native, x, y, 2. * (y + x)
);

bench!(bench3, bench3_id, bench3_noset, bench3_unsafe,
    "(2 * y + 2 * x)",
    bench3_native, x, y,
    2. * y + 2. * x
);

bench!(bench4, bench4_id, bench4_noset, bench4_unsafe,
    "((1.23 * x^2) / y) - 123.123",
    bench4_native, x, y,
    ((1.23 * x.powf(2.)) / y) - 123.123
);

bench!(bench5, bench5_id, bench5_noset, bench5_unsafe,
    "(y + x / y) * (x - y / x)",
    bench5_native, x, y,
    (y + x / y) * (x - y / x)
);

bench!(bench6, bench6_id, bench6_noset, bench6_unsafe,
    "x / ((x + y) + (x - y)) / y",
    bench6_native, x, y,
    x / ((x + y) + (x - y)) / y
);

bench!(bench7, bench7_id, bench7_noset, bench7_unsafe,
    "1 - ((x * y) + (y / x)) - 3",
    bench7_native, x, y,
    1. - ((x * y) + (y / x)) - 3.
);

bench!(bench8, bench8_id, bench8_noset, bench8_unsafe,
    "(5.5 + x) + (2 * x - 2 / 3 * y) * (x / 3 + y / 4) + (y + 7.7)",
    bench8_native, x, y,
    (5.5 + x) + (2. * x - 2. / 3. * y) * (x / 3. + y / 4.) + (y + 7.7)
);

bench!(bench9, bench9_id, bench9_noset, bench9_unsafe,
    "1.1x^1 + 2.2y^2 - 3.3x^3 + 4.4y^15 - 5.5x^23 + 6.6y^55",
    bench9_native, x, y,
    1.1*x.powf(1.) + 2.2*y.powf(2.) - 3.3*x.powf(3.) + 4.4*y.powf(15.) - 5.5*x.powf(23.) + 6.6*y.powf(55.)
);

bench!(bench10, bench10_id, bench10_noset, bench10_unsafe,
    "sin(2 * x) + cos(pi / y)",
    bench10_native, x, y,
    (2. * x).sin() + (PI / y).cos()
);

bench!(bench11, bench11_id, bench11_noset, bench11_unsafe,
    "1 - sin(2 * x) + cos(pi / y)",
    bench11_native, x, y,
    1. - (2. * x).sin() + (PI / y).cos()
);

bench!(bench12, bench12_id, bench12_noset, bench12_unsafe,
    "sqrt(111.111 - sin(2 * x) + cos(pi / y) / 333.333)",
    bench12_native, x, y,
    (111.111 - (2. * x).sin() + (PI / y).sin() / 333.333).sqrt()
);

bench!(bench13, bench13_id, bench13_noset, bench13_unsafe,
    "(x^2 / sin(2 * pi / y)) - x / 2",
    bench13_native, x, y,
    (x.powf(2.) / (2. * PI / y).sin()) - x / 2.
);

bench!(bench14, bench14_id, bench14_noset, bench14_unsafe,
    "x + (cos(y - sin(2 / x * pi)) - sin(x - cos(2 * y / pi))) - y",
    bench14_native, x, y,
    x + ((y - (2. / x * PI).sin()).cos() - (x - (2. * y / PI).cos()).sin()) - y
);

bench!(bench16, bench16_id, bench16_noset, bench16_unsafe,
    "max(3.33, min(sqrt(1 - sin(2 * x) + cos(pi / y) / 3), 1.11))",
    bench16_native, x, y,
    (3.33 as c_double).max((1.11 as c_double).min(1. - (2. * x).sin() + (PI / y).cos() / 3.).sqrt())
);

bench!(bench17, bench17_id, bench17_noset, bench17_unsafe,
    "if((y + (x * 2.2)) <= (x + y + 1.1), x - y, x * y) + 2 * pi / x",
    bench17_native, x, y,
    (if (y + (x * 2.2)) <= (x + y + 1.1) { x - y } else { x * y }) + 2. * PI / x
);
