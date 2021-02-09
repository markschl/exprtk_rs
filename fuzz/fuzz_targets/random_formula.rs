#![no_main]

mod validation;

libfuzzer_sys::fuzz_target!(|data: validation::Data| {
    validation::validate_input(data)
});
