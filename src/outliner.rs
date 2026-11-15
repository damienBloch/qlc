//! The outliner is the third stage. It turns the AST into the host IR.
//!
//! It takes each `on` block out of its function and sends its body to its target,
//! which compiles it to a kernel. In the host IR, the `on` block is replaced by a call
//! to this kernel.

use std::fmt;

use crate::ast;
use crate::host_ir::{self, KernelId};
use crate::targets::{Kernel, Registry, TargetError};

#[derive(Debug, PartialEq, Eq)]
pub enum OutlineError {
    /// An `on` block uses a target that is not in the registry.
    UnknownTarget(String),
    /// The target of an `on` block could not compile it.
    Target { target: String, error: TargetError },
}

impl fmt::Display for OutlineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutlineError::UnknownTarget(name) => write!(f, "unknown target `{name}`"),
            OutlineError::Target { target, error } => write!(f, "target `{target}`: {error}"),
        }
    }
}

/// Outlines the `on` blocks of `program`. They are compiled by the targets of `targets`.
///
/// The `on` blocks of all functions are compiled, even if the function is never called.
pub fn outline(
    program: &ast::Program,
    targets: &Registry,
) -> Result<host_ir::Program, OutlineError> {
    let mut kernels = Vec::new();
    let functions = program
        .functions
        .iter()
        .map(|function| outline_function(function, targets, &mut kernels))
        .collect::<Result<_, _>>()?;
    Ok(host_ir::Program::new(functions, kernels))
}

/// Outlines the `on` blocks of `function`. Their kernels are added to `kernels`.
fn outline_function(
    function: &ast::Function,
    targets: &Registry,
    kernels: &mut Vec<Box<dyn Kernel>>,
) -> Result<host_ir::Function, OutlineError> {
    let body = function
        .body
        .iter()
        .map(|statement| outline_statement(statement, targets, kernels))
        .collect::<Result<_, _>>()?;
    Ok(host_ir::Function {
        name: function.name.clone(),
        body,
    })
}

fn outline_statement(
    statement: &ast::Statement,
    targets: &Registry,
    kernels: &mut Vec<Box<dyn Kernel>>,
) -> Result<host_ir::Statement, OutlineError> {
    match statement {
        ast::Statement::Print(value) => Ok(host_ir::Statement::Print(*value)),
        ast::Statement::On { target, body } => {
            let kernel = targets
                .get(target)
                .ok_or_else(|| OutlineError::UnknownTarget(target.clone()))?
                .compile(body)
                .map_err(|error| OutlineError::Target {
                    target: target.clone(),
                    error,
                })?;
            let id = KernelId(kernels.len());
            kernels.push(kernel);
            Ok(host_ir::Statement::CallKernel(id))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;
    use crate::targets::Target;

    /// A target whose kernels write the statements they were compiled from,
    /// such as `fake: [Print(1)]`.
    struct FakeTarget;

    impl Target for FakeTarget {
        fn compile(&self, body: &[ast::Statement]) -> Result<Box<dyn Kernel>, TargetError> {
            Ok(Box::new(FakeKernel {
                text: format!("fake: {body:?}"),
            }))
        }
    }

    struct FakeKernel {
        text: String,
    }

    impl Kernel for FakeKernel {
        fn call(&self, output: &mut dyn Write) {
            writeln!(output, "{}", self.text).expect("writing to a Vec should succeed");
        }
    }

    /// A target that never compiles.
    struct FailingTarget;

    impl Target for FailingTarget {
        fn compile(&self, _body: &[ast::Statement]) -> Result<Box<dyn Kernel>, TargetError> {
            Err(TargetError("cannot compile".to_string()))
        }
    }

    /// Outlines `program` with two targets: `fake` and `failing`.
    fn outline_with_test_targets(program: &ast::Program) -> Result<host_ir::Program, OutlineError> {
        let mut targets = Registry::empty();
        targets.add("fake", Box::new(FakeTarget));
        targets.add("failing", Box::new(FailingTarget));
        outline(program, &targets)
    }

    /// Calls the kernel `id` of `program` and returns what it printed.
    fn call_to_string(program: &host_ir::Program, id: KernelId) -> String {
        let mut output = Vec::new();
        program
            .kernel(id)
            .expect("the test should only call kernels that the outliner created")
            .call(&mut output);
        String::from_utf8(output).expect("the output should be UTF-8")
    }

    /// A program with only a `main` function, whose body is `body`.
    fn main_with_body(body: Vec<ast::Statement>) -> ast::Program {
        ast::Program {
            functions: vec![ast::Function {
                name: "main".to_string(),
                body,
            }],
        }
    }

    /// An `on` block that runs `body` on `target`.
    fn on(target: &str, body: Vec<ast::Statement>) -> ast::Statement {
        ast::Statement::On {
            target: target.to_string(),
            body,
        }
    }

    /// Returns the body of the only function of `program`.
    fn compiled_body(program: &host_ir::Program) -> &[host_ir::Statement] {
        let [function] = program.functions() else {
            panic!("test program should have one function");
        };
        &function.body
    }

    #[test]
    fn print_is_kept() {
        let program = outline_with_test_targets(&main_with_body(vec![ast::Statement::Print(1)]))
            .expect("the program should compile");
        assert_eq!(compiled_body(&program), [host_ir::Statement::Print(1)]);
    }

    #[test]
    fn on_block_is_replaced_by_a_kernel_call() {
        let body = vec![
            ast::Statement::Print(1),
            on("fake", vec![ast::Statement::Print(2)]),
            ast::Statement::Print(3),
        ];
        let program =
            outline_with_test_targets(&main_with_body(body)).expect("the program should compile");
        assert_eq!(
            compiled_body(&program),
            [
                host_ir::Statement::Print(1),
                host_ir::Statement::CallKernel(KernelId(0)),
                host_ir::Statement::Print(3),
            ]
        );
        assert_eq!(call_to_string(&program, KernelId(0)), "fake: [Print(2)]\n");
    }

    #[test]
    fn each_on_block_gets_its_own_kernel_in_order() {
        let body = vec![
            on("fake", vec![ast::Statement::Print(1)]),
            on("fake", vec![ast::Statement::Print(2)]),
        ];
        let program =
            outline_with_test_targets(&main_with_body(body)).expect("the program should compile");
        assert_eq!(
            compiled_body(&program),
            [
                host_ir::Statement::CallKernel(KernelId(0)),
                host_ir::Statement::CallKernel(KernelId(1)),
            ]
        );
        assert_eq!(call_to_string(&program, KernelId(0)), "fake: [Print(1)]\n");
        assert_eq!(call_to_string(&program, KernelId(1)), "fake: [Print(2)]\n");
    }

    #[test]
    fn unknown_target_is_an_error() {
        let body = vec![ast::Statement::Print(1), on("unknown", vec![])];
        assert_eq!(
            outline_with_test_targets(&main_with_body(body)).err(),
            Some(OutlineError::UnknownTarget("unknown".to_string()))
        );
    }

    #[test]
    fn unknown_target_in_a_function_that_is_not_called_is_an_error() {
        let program = ast::Program {
            functions: vec![
                ast::Function {
                    name: "main".to_string(),
                    body: vec![],
                },
                ast::Function {
                    name: "helper".to_string(),
                    body: vec![on("unknown", vec![])],
                },
            ],
        };
        assert_eq!(
            outline_with_test_targets(&program).err(),
            Some(OutlineError::UnknownTarget("unknown".to_string()))
        );
    }

    #[test]
    fn target_error_is_returned_with_the_target_name() {
        let body = vec![on("failing", vec![])];
        assert_eq!(
            outline_with_test_targets(&main_with_body(body)).err(),
            Some(OutlineError::Target {
                target: "failing".to_string(),
                error: TargetError("cannot compile".to_string()),
            })
        );
    }

    #[test]
    fn target_error_message() {
        let error = OutlineError::Target {
            target: "failing".to_string(),
            error: TargetError("cannot compile".to_string()),
        };
        assert_eq!(error.to_string(), "target `failing`: cannot compile");
    }
}
