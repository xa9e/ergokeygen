mod cli;

use std::process;

fn main() {
    if let Err(err) = cli::run() {
        eprintln!("error: {err}\n");
        cli::print_usage();
        process::exit(2);
    }
}
