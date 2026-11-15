//! The interpreter is the last stage. It executes a program from its host IR.

use std::collections::HashSet;
use std::fmt;
use std::io::Write;

use crate::host_ir::{Function, Program, Statement};

#[derive(Debug, PartialEq, Eq)]
pub enum RunError {
    MissingMain,
    DuplicateFunction(String),
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunError::MissingMain => write!(f, "no `main` function found"),
            RunError::DuplicateFunction(name) => {
                write!(f, "function `{name}` is defined more than once")
            }
        }
    }
}

/// Runs the `main` function of `program`. What the program prints is written to `output`.
///
/// The program is checked before it runs, so an error is returned before anything is printed.
pub fn run(program: &Program, output: &mut impl Write) -> Result<(), RunError> {
    check_no_duplicate_functions(program)?;
    let main = program
        .functions()
        .iter()
        .find(|function| function.name == "main")
        .ok_or(RunError::MissingMain)?;
    call(program, main, output);
    Ok(())
}

fn check_no_duplicate_functions(program: &Program) -> Result<(), RunError> {
    let mut names = HashSet::new();
    for function in program.functions() {
        if !names.insert(function.name.as_str()) {
            return Err(RunError::DuplicateFunction(function.name.clone()));
        }
    }
    Ok(())
}

/// Calls `function`, which is a function of `program`.
fn call(program: &Program, function: &Function, output: &mut impl Write) {
    for statement in &function.body {
        match statement {
            Statement::Print(value) => {
                writeln!(output, "{value}").expect("writing the program output should succeed")
            }
            Statement::CallKernel(id) => program
                .kernel(*id)
                .expect("the outliner should create a kernel for each `CallKernel` statement")
                .call(output),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host_ir::KernelId;
    use crate::targets::Kernel;

    /// A kernel that writes `kernel <name>`.
    struct FakeKernel {
        name: &'static str,
    }

    impl Kernel for FakeKernel {
        fn call(&self, output: &mut dyn Write) {
            writeln!(output, "kernel {}", self.name).expect("writing to a Vec should succeed");
        }
    }

    /// Runs `program` and returns what it printed.
    fn run_to_string(program: &Program) -> Result<String, RunError> {
        let mut output = Vec::new();
        run(program, &mut output)?;
        Ok(String::from_utf8(output).expect("the output should be UTF-8"))
    }

    /// A program with only a `main` function, whose body is `body`, and the kernels `kernels`.
    fn main_with_body_and_kernels(body: Vec<Statement>, kernels: Vec<Box<dyn Kernel>>) -> Program {
        let main = Function {
            name: "main".to_string(),
            body,
        };
        Program::new(vec![main], kernels)
    }

    /// A program with only a `main` function, whose body is `body`, and no kernels.
    fn main_with_body(body: Vec<Statement>) -> Program {
        main_with_body_and_kernels(body, vec![])
    }

    /// A program with empty functions named `function_names`, and no kernels.
    fn program(function_names: &[&str]) -> Program {
        let functions = function_names
            .iter()
            .map(|name| Function {
                name: name.to_string(),
                body: vec![],
            })
            .collect();
        Program::new(functions, vec![])
    }

    #[test]
    fn runs_program_with_main() {
        assert_eq!(run_to_string(&program(&["main"])), Ok(String::new()));
    }

    #[test]
    fn runs_program_with_main_and_other_functions() {
        assert_eq!(
            run_to_string(&program(&["helper", "main"])),
            Ok(String::new())
        );
    }

    #[test]
    fn print_writes_the_value_on_its_own_line() {
        assert_eq!(
            run_to_string(&main_with_body(vec![Statement::Print(42)])),
            Ok("42\n".to_string())
        );
    }

    #[test]
    fn statements_run_in_order() {
        let body = vec![
            Statement::Print(1),
            Statement::Print(i64::MAX),
            Statement::Print(0),
        ];
        assert_eq!(
            run_to_string(&main_with_body(body)),
            Ok("1\n9223372036854775807\n0\n".to_string())
        );
    }

    #[test]
    fn call_kernel_calls_the_kernel_with_this_id() {
        let program = main_with_body_and_kernels(
            vec![
                Statement::Print(1),
                Statement::CallKernel(KernelId(1)),
                Statement::CallKernel(KernelId(0)),
                Statement::CallKernel(KernelId(1)),
            ],
            vec![
                Box::new(FakeKernel { name: "a" }),
                Box::new(FakeKernel { name: "b" }),
            ],
        );
        assert_eq!(
            run_to_string(&program),
            Ok("1\nkernel b\nkernel a\nkernel b\n".to_string())
        );
    }

    #[test]
    #[should_panic(
        expected = "the outliner should create a kernel for each `CallKernel` statement"
    )]
    fn call_of_a_missing_kernel_panics() {
        let program = main_with_body(vec![Statement::CallKernel(KernelId(0))]);
        let _ = run_to_string(&program);
    }

    #[test]
    fn missing_main_is_an_error() {
        assert_eq!(
            run_to_string(&program(&["helper"])),
            Err(RunError::MissingMain)
        );
    }

    #[test]
    fn duplicate_function_is_an_error() {
        assert_eq!(
            run_to_string(&program(&["main", "main"])),
            Err(RunError::DuplicateFunction("main".to_string()))
        );
    }
}
