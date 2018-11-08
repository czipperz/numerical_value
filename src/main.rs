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

use std::io;
use std::fs;

fn parse_args() -> io::Result<(String, String)> {
    let args: Vec<String> = ::std::env::args().skip(1).collect();
    if args.len() == 0 {
        Err(io::Error::new(io::ErrorKind::InvalidInput, "No file_in argument"))
    } else if args.len() == 1 {
        Err(io::Error::new(io::ErrorKind::InvalidInput, "No file_out argument"))
    } else if args.len() == 2 {
        let mut iter = args.into_iter();
        let file_in = iter.next().unwrap();
        let file_out = iter.next().unwrap();
        Ok((file_in, file_out))
    } else {
        Err(io::Error::new(io::ErrorKind::InvalidInput,
                           "Too many arguments.  Should be just file_in and file_out"))
    }
}

fn main_() -> io::Result<()> {
    let (file_in, file_out) = parse_args()?;
    let graph = parse(&file_in)?;
    let diagnostics = analyze(&graph);
    let diagnostics: String = serde_json::to_string_pretty(&diagnostics)?;
    println!("\n{}", diagnostics);
    fs::write(&file_out, &diagnostics)?;
    Ok(())
}

fn main() {
    match main_() {
        Ok(_) => {},
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        },
    }
}
