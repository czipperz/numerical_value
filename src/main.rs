pub mod numerical_value;
pub use numerical_value::*;
mod parse;
use parse::*;

#[macro_use]
extern crate serde_derive;

fn main() {
    let graph = parse();
    println!("{:?}", graph);
}
