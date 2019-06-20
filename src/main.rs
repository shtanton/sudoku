use std::env;
use std::fs;
use std::io::{self, Read};
use std::process;

use sudoku;

const USAGE_INFO: &str =
    "Usage:\n * sudoku FORMAT_FILE SUDOKU_FILE\nOr stdin\n * sudoku FORMAT_FILE";

struct Config {
    sud: String,
    fmt_fname: String,
}

impl Config {
    fn new(args: Vec<String>) -> Result<Config, String> {
        if args.len() == 1 {
            Err(USAGE_INFO.to_string())
        } else if args.len() == 2 {
            let mut sud: String = String::new();
            io::stdin()
                .read_to_string(&mut sud)
                .map_err(|_| "Error reading from stdin")?;
            sud = sud.trim().to_string();
            if sud == "" {
                Err(USAGE_INFO.to_string())
            } else {
                Ok(Config {
                    sud: sud,
                    fmt_fname: args[1].clone(),
                })
            }
        } else {
            let sud = fs::read_to_string(&args[2])
                .map_err(|_| "Error reading sudoku file")?
                .trim()
                .to_string();
            Ok(Config {
                sud,
                fmt_fname: args[1].clone(),
            })
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let config: Config = Config::new(args).unwrap_or_else(|err| {
        println!("Error:\n{}", err);
        process::exit(1);
    });

    if let Err(err) = sudoku::run(config.sud, config.fmt_fname) {
        println!("Error:\n{}", err);
        process::exit(1);
    };
}
