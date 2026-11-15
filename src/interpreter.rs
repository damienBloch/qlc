//! The interpreter is the last stage. It executes a program from its host IR.
//!
//! The previous stages check the program, so running it cannot fail.

use std::io::Write;

use crate::host_ir::{Function, Program, Statement};

/// Runs the `main` function of `program`. What the program prints is written to `output`.
pub fn run(program: &Program, output: &mut impl Write) {
    call(program, program.main(), output);
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
    use crate::resolved_ast::FunctionId;
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
    fn run_to_string(program: &Program) -> String {
        let mut output = Vec::new();
        run(program, &mut output);
        String::from_utf8(output).expect("the output should be UTF-8")
    }

    fn function(name: &str, body: Vec<Statement>) -> Function {
        Function {
            name: name.to_string(),
            body,
        }
    }

    /// A program with only a `main` function, whose body is `body`, and the kernels `kernels`.
    fn main_with_body_and_kernels(body: Vec<Statement>, kernels: Vec<Box<dyn Kernel>>) -> Program {
        Program::new(vec![function("main", body)], FunctionId(0), kernels)
    }

    /// A program with only a `main` function, whose body is `body`, and no kernels.
    fn main_with_body(body: Vec<Statement>) -> Program {
        main_with_body_and_kernels(body, vec![])
    }

    #[test]
    fn runs_empty_main() {
        assert_eq!(run_to_string(&main_with_body(vec![])), "");
    }

    #[test]
    fn runs_only_the_main_function() {
        let program = Program::new(
            vec![
                function("helper", vec![Statement::Print(1)]),
                function("main", vec![Statement::Print(2)]),
            ],
            FunctionId(1),
            vec![],
        );
        assert_eq!(run_to_string(&program), "2\n");
    }

    #[test]
    fn print_writes_the_value_on_its_own_line() {
        assert_eq!(
            run_to_string(&main_with_body(vec![Statement::Print(42)])),
            "42\n"
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
            "1\n9223372036854775807\n0\n"
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
        assert_eq!(run_to_string(&program), "1\nkernel b\nkernel a\nkernel b\n");
    }

    #[test]
    #[should_panic(
        expected = "the outliner should create a kernel for each `CallKernel` statement"
    )]
    fn call_of_a_missing_kernel_panics() {
        let program = main_with_body(vec![Statement::CallKernel(KernelId(0))]);
        run_to_string(&program);
    }

    #[test]
    #[should_panic(expected = "the `main` function should be one of the functions of the program")]
    fn program_with_an_invalid_main_id_panics() {
        Program::new(vec![], FunctionId(0), vec![]);
    }
}
