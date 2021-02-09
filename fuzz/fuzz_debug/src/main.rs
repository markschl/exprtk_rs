
use std::env::args;
use std::fs::File;
use std::io::Read;
use arbitrary::{Arbitrary, Unstructured};

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
    
    let mut unstructured = Unstructured::new(data.as_slice());
    let data = Arbitrary::arbitrary(&mut unstructured).unwrap();
    println!("{:?}", data);

    validation::validate_input(data);
}
