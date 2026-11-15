//! Integration tests. They run the `qlc` binary on the programs in `tests/programs`
//! and only check its exit code and its output.

use std::process::{Command, Output};

/// Runs `qlc` on the file `tests/programs/<name>`.
fn run_program(name: &str) -> Output {
    let path = format!("{}/tests/programs/{name}", env!("CARGO_MANIFEST_DIR"));
    Command::new(env!("CARGO_BIN_EXE_qlc"))
        .arg(path)
        .output()
        .expect("the qlc binary should start")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be UTF-8")
}

/// Checks that the program ran successfully and printed nothing.
fn assert_success(output: &Output) {
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(output));
    assert_eq!(stdout(output), "");
    assert_eq!(stderr(output), "");
}

/// Checks that qlc reported `message` as an error.
/// The exit code must be exactly 1, because a panic exits with a different code.
fn assert_error(output: &Output, message: &str) {
    assert_eq!(output.status.code(), Some(1), "stderr: {}", stderr(output));
    assert_eq!(stdout(output), "");
    assert_eq!(stderr(output).trim_end(), format!("error: {message}"));
}

#[test]
fn empty_main() {
    assert_success(&run_program("empty_main.ql"));
}

#[test]
fn missing_main() {
    assert_error(&run_program("missing_main.ql"), "no `main` function found");
}

#[test]
fn syntax_error() {
    assert_error(
        &run_program("missing_closing_brace.ql"),
        "expected `}`, found end of input",
    );
}

#[test]
fn unknown_character() {
    assert_error(
        &run_program("unknown_character.ql"),
        "unexpected character `$`",
    );
}

#[test]
fn missing_file() {
    let output = run_program("does_not_exist.ql");
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).starts_with("error: cannot read"));
}
