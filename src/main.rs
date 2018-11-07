mod bounded_value;

mod numerical_value;
use numerical_value::*;

mod numerical_value_analysis;
use numerical_value_analysis::*;

mod parse;
use parse::*;

#[macro_use]
extern crate serde_derive;
extern crate serde_json;

fn main() {
    match parse() {
        Ok(graph) => {
            let diagnostics = analyze(&graph);
            let diagnostics: String = serde_json::to_string(&diagnostics).unwrap();
            println!("{}", diagnostics);
        },
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        },
    }
}
