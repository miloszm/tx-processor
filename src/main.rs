mod accounts;
mod dispatcher;
mod driver;
mod error;
mod parser;
mod types;

use std::env;
use std::process;

fn main() {
    let mut args = env::args().skip(1); // skip program name

    let file_path = match args.next() {
        Some(path) => path,
        None => {
            eprintln!("Usage: {} <file_path>", env::args().next().unwrap());
            process::exit(1);
        }
    };

    if args.next().is_some() {
        eprintln!("Error: too many arguments. Expected exactly one file path.");
        process::exit(1);
    }

    let result = driver::TxProcessor::run(&file_path);
    if let Err(e) = result {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}
