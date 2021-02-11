use arbitrary::{Arbitrary, Unstructured};
use std::env::args;
use std::fs::File;
use std::io::Read;

mod validation {
    include!("../../fuzz_targets/validation.rs");
}

fn main() {
    let mut data = vec![];
    let filename = args().skip(1).next().unwrap().as_str().to_string();
    File::open(&filename)
        .unwrap()
        .read_to_end(&mut data)
        .expect("could not open file");

    let data = Arbitrary::arbitrary_take_rest(Unstructured::new(data.as_slice())).unwrap();
    println!("{:?}", data);

    validation::validate_input(data);
}
