mod cli;

use std::process;

fn main() {
    if let Err(err) = cli::run() {
        if cli::is_broken_pipe_error(&err) {
            return;
        }
        eprintln!("error: {err}\n");
        cli::print_usage();
        process::exit(2);
    }
}
