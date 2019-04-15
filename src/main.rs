use std::env;
use std::process;

use sudoku;

struct Config {
    sud_fname: String,
    fmt_fname: String,
}

impl Config {
    fn new(args: Vec<String>) -> Option<Config> {
        if args.len() < 3 {
            None
        } else {
            Some(Config {sud_fname: args[1].clone(), fmt_fname: args[2].clone()})
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let config: Config = Config::new(args).unwrap_or_else(|| {
        println!("Usage: sudoku SUDFILE FMTFILE");
        process::exit(1);
    });

    if let Err(err) = sudoku::run(&config.sud_fname, &config.fmt_fname) {
        println!("Error: {}", err);
        process::exit(1);
    };
}
