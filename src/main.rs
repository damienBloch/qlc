//! Command line entry point: `qlc <file>`.
//!
//! Exit codes:
//! - 0: the program ran successfully.
//! - 1: there was an error. The error is printed on stderr.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let [_, path] = args.as_slice() else {
        eprintln!("usage: qlc <file>");
        return ExitCode::FAILURE;
    };

    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("error: cannot read `{path}`: {error}");
            return ExitCode::FAILURE;
        }
    };

    match qlc::run_source(&source) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
