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

/// Checks that the program ran successfully and printed `expected_stdout`.
fn assert_success(output: &Output, expected_stdout: &str) {
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(output));
    assert_eq!(stdout(output), expected_stdout);
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
    assert_success(&run_program("empty_main.ql"), "");
}

#[test]
fn print_integer() {
    assert_success(&run_program("print_integer.ql"), "42\n");
}

#[test]
fn print_several_integers() {
    assert_success(&run_program("print_several_integers.ql"), "1\n2\n3\n");
}

#[test]
fn only_main_runs() {
    assert_success(&run_program("only_main_runs.ql"), "1\n");
}

#[test]
fn variables() {
    assert_success(
        &run_program("variables.ql"),
        "1
1
2
1
",
    );
}

#[test]
fn unknown_variable() {
    assert_error(&run_program("unknown_variable.ql"), "unknown variable `x`");
}

#[test]
fn sim_device_in_a_variable() {
    assert_success(&run_program("sim_device.ql"), "1\n");
}

#[test]
fn print_of_a_sim_device() {
    assert_error(
        &run_program("print_sim_device.ql"),
        "expected a value of type `Int`, found `SimDevice`",
    );
}

#[test]
fn delete_of_an_integer() {
    assert_error(
        &run_program("delete_integer.ql"),
        "expected a value of type `SimDevice`, found `Int`",
    );
}

#[test]
fn use_of_a_deleted_sim_device() {
    assert_error(
        &run_program("delete_twice.ql"),
        "variable `d` is used after its value is moved",
    );
}

#[test]
fn integer_too_large() {
    assert_error(
        &run_program("integer_too_large.ql"),
        "integer `9223372036854775808` is too large",
    );
}

#[test]
fn missing_main() {
    assert_error(&run_program("missing_main.ql"), "no `main` function found");
}

#[test]
fn duplicate_function() {
    assert_error(
        &run_program("duplicate_function.ql"),
        "function `helper` is defined more than once",
    );
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
