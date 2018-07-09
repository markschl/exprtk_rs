#![feature(test)]
#![allow(unused_variables)]

extern crate test;
extern crate exprtk_rs;

use std::f64::consts::PI;
use exprtk_rs::*;
use self::test::Bencher;


const XMIN: c_double = -100.;
const XMAX: c_double = 100.;
const YMIN: c_double = -100.;
const YMAX: c_double = 100.;
const DELTA: c_double = 0.0111;


macro_rules! bench {
    ($name:ident, $name_noset:ident, $name_set_pointer:ident, $formula:expr, $name_native:ident,
        $x:ident, $y:ident, $expr:expr) => {
        // "Normal" usage of API
        #[bench]
        fn $name(b: &mut Bencher) {
            let mut s = SymbolTable::new();
            s.add_pi();
            let x_id = s.add_variable("x", 0.).unwrap().unwrap();
            let y_id = s.add_variable("y", 0.).unwrap().unwrap();
            let mut e = Expression::new($formula, s).unwrap();
            b.iter(|| {
                let mut total = 0.;
                let mut x = XMIN;
                let mut y = YMIN;
                while x < XMAX {
                    x += DELTA;
                    while y < YMAX {
                        y += DELTA;
                        e.symbols().set_value(x_id, x as c_double);
                        e.symbols().set_value(y_id, y as c_double);
                        total += e.value();
                    }
                }
            });
        }

        // Simulating the behaviour in C++ (as good as possible):
        // The pointers are directly incremented
        #[bench]
        fn $name_set_pointer(b: &mut Bencher) {
            let mut s = SymbolTable::new();
            s.add_pi();
            let x_id = s.add_variable("x", 0.).unwrap().unwrap();
            let y_id = s.add_variable("y", 0.).unwrap().unwrap();
            let x_ptr = s.get_value_ptr(x_id).unwrap();
            let y_ptr = s.get_value_ptr(y_id).unwrap();
            let e = Expression::new($formula, s).unwrap();
            b.iter(|| {
                let mut total = 0.;
                unsafe {
                    *x_ptr = XMIN;
                    *y_ptr = YMIN;
                    while *x_ptr < XMAX {
                        *x_ptr += DELTA;
                        while *y_ptr < YMAX  {
                            *y_ptr += DELTA;
                            total += e.value();
                        }
                    }
                }
            });
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



bench!(bench1, bench1_noset, bench1_ptr,
    "(y + x)",
    bench1_native, x, y, x + y
);

bench!(bench2, bench2_noset, bench2_ptr,
    "2 * (y + x)",
    bench2_native, x, y, 2. * (y + x)
);

bench!(bench3, bench3_noset, bench3_ptr,
    "(2 * y + 2 * x)",
    bench3_native, x, y,
    2. * y + 2. * x
);

bench!(bench4, bench4_noset, bench4_ptr,
    "((1.23 * x^2) / y) - 123.123",
    bench4_native, x, y,
    ((1.23 * x.powf(2.)) / y) - 123.123
);

bench!(bench5, bench5_noset, bench5_ptr,
    "(y + x / y) * (x - y / x)",
    bench5_native, x, y,
    (y + x / y) * (x - y / x)
);

bench!(bench6, bench6_noset, bench6_ptr,
    "x / ((x + y) + (x - y)) / y",
    bench6_native, x, y,
    x / ((x + y) + (x - y)) / y
);

bench!(bench7, bench7_noset, bench7_ptr,
    "1 - ((x * y) + (y / x)) - 3",
    bench7_native, x, y,
    1. - ((x * y) + (y / x)) - 3.
);

bench!(bench8, bench8_noset, bench8_ptr,
    "(5.5 + x) + (2 * x - 2 / 3 * y) * (x / 3 + y / 4) + (y + 7.7)",
    bench8_native, x, y,
    (5.5 + x) + (2. * x - 2. / 3. * y) * (x / 3. + y / 4.) + (y + 7.7)
);

bench!(bench9, bench9_noset, bench9_ptr,
    "1.1x^1 + 2.2y^2 - 3.3x^3 + 4.4y^15 - 5.5x^23 + 6.6y^55",
    bench9_native, x, y,
    1.1*x.powf(1.) + 2.2*y.powf(2.) - 3.3*x.powf(3.) + 4.4*y.powf(15.) - 5.5*x.powf(23.) + 6.6*y.powf(55.)
);

bench!(bench10, bench10_noset, bench10_ptr,
    "sin(2 * x) + cos(pi / y)",
    bench10_native, x, y,
    (2. * x).sin() + (PI / y).sin()
);

bench!(bench11, bench11_noset, bench11_ptr,
    "1 - sin(2 * x) + cos(pi / y)",
    bench11_native, x, y,
    1. - (2. * x).sin() + (PI / y).sin()
);

bench!(bench12, bench12_noset, bench12_ptr,
    "sqrt(111.111 - sin(2 * x) + cos(pi / y) / 333.333)",
    bench12_native, x, y,
    (111.111 - (2. * x).sin() + (PI / y).sin() / 333.333).sqrt()
);

bench!(bench13, bench13_noset, bench13_ptr,
    "(x^2 / sin(2 * pi / y)) - x / 2",
    bench13_native, x, y,
    (x.powf(2.) / (2. * PI / y).sin()) - x / 2.
);

bench!(bench14, bench14_noset, bench14_ptr,
    "x + (cos(y - sin(2 / x * pi)) - sin(x - cos(2 * y / pi))) - y",
    bench14_native, x, y,
    x + ((y - (2. / x * PI).sin()).cos() - (x - (2. * y / PI).cos()).sin()) - y
);

bench!(bench16, bench16_noset, bench16_ptr,
    "max(3.33, min(sqrt(1 - sin(2 * x) + cos(pi / y) / 3), 1.11))",
    bench16_native, x, y,
    (3.33 as c_double).max((1.11 as c_double).min(1. - (2. * x).sin() + (PI / y).cos() / 3.).sqrt())
);

bench!(bench17, bench17_noset, bench17_ptr,
    "if((y + (x * 2.2)) <= (x + y + 1.1), x - y, x * y) + 2 * pi / x",
    bench17_native, x, y,
    (if (y + (x * 2.2)) <= (x + y + 1.1) { x - y } else { x * y }) + 2. * PI / x
);
