mod bounded_value;

mod numerical_value;
use numerical_value::*;

mod numerical_value_analysis;
use numerical_value_analysis::*;

mod parse;
use parse::*;

#[macro_use]
extern crate serde_derive;

fn main() {
    let graph = parse().unwrap();
    analyze(&graph);
}
