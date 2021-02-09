Randomly generated formulas with random variable names are used as input for testing different
aspects of the API:

* Adding variables
* compiling and evaluating formulae
* Manipulating symbol tables
* Cloning different types

# Setup

Install cargo-fuzz:

```sh
cargo install cargo-fuzz
```

# Running

```sh
num_jobs=4
cargo fuzz run -j $num_jobs --release random_formula -- -only_ascii=1
```

# Debugging

If a problem was found, it may need to be minified, here an example:

```sh
cargo fuzz tmin --release random_formula fuzz/artifacts/random_formula/crash-0a7cc920d077cd5454a397fbe8fd5833509c5086
```

Then, the minified input was analysed using a separate test crate. There may
be an easier way that I'm unaware of, but this works:

```sh
(cd fuzz/fuzz_debug && cargo run ../artifacts/random_formula/minimized-from-a7d26fe5cfaea49fb041c2f4fa8ca2e811c362ca)
```
