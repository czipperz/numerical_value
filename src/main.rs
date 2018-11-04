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
    match parse() {
        Ok(graph) => analyze(&graph),
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        },
    }
}
