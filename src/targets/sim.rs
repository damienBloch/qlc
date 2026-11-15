//! `sim` is a simulated device. It is used to test how the host runs code on devices.
//!
//! It works in two steps:
//! - [`generate`] turns the body of an `on` block into a [`Program`], the IR of this device.
//! - [`simulate`] runs a [`Program`]. Each line that the device prints starts with `sim: `.

use std::fmt;
use std::io::Write;

use crate::ast::Statement;
use crate::targets::{Kernel, Target, TargetError};

/// The target that compiles and simulates `on sim` blocks.
pub struct SimTarget;

impl Target for SimTarget {
    fn compile(&self, body: &[Statement]) -> Result<Box<dyn Kernel>, TargetError> {
        Ok(Box::new(SimKernel {
            program: generate(body),
        }))
    }
}

/// An `on sim` block, compiled to a `sim` program.
struct SimKernel {
    program: Program,
}

impl Kernel for SimKernel {
    fn call(&self, output: &mut dyn Write) {
        simulate(&self.program, output);
    }
}

/// A program for the `sim` device.
#[derive(Debug, PartialEq, Eq)]
pub struct Program {
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Instruction {
    /// Prints an integer on its own line.
    Print(i64),
}

/// Writes one instruction per line, such as `print 42`.
impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for instruction in &self.instructions {
            match instruction {
                Instruction::Print(value) => writeln!(f, "print {value}")?,
            }
        }
        Ok(())
    }
}

/// Generates a `sim` program from the body of an `on sim` block.
pub fn generate(body: &[Statement]) -> Program {
    let instructions = body
        .iter()
        .map(|statement| match statement {
            Statement::Print(value) => Instruction::Print(*value),
            Statement::On { .. } => todo!("run an `on` block from inside a device (chaining)"),
        })
        .collect();
    Program { instructions }
}

/// Runs `program` on the simulated device. What it prints is written to `output`.
pub fn simulate(program: &Program, output: &mut dyn Write) {
    for instruction in &program.instructions {
        match instruction {
            Instruction::Print(value) => {
                writeln!(output, "sim: {value}").expect("writing the device output should succeed")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Simulates `program` and returns what it printed.
    fn simulate_to_string(program: &Program) -> String {
        let mut output = Vec::new();
        simulate(program, &mut output);
        String::from_utf8(output).expect("the output should be UTF-8")
    }

    #[test]
    fn generate_print_statements() {
        let body = vec![Statement::Print(1), Statement::Print(2)];
        assert_eq!(generate(&body).to_string(), "print 1\nprint 2\n");
    }

    #[test]
    fn generate_empty_body() {
        assert_eq!(generate(&[]).to_string(), "");
    }

    #[test]
    #[should_panic(expected = "chaining")]
    fn generate_nested_on_block_is_not_supported_yet() {
        let body = vec![Statement::On {
            target: "sim".to_string(),
            body: vec![],
        }];
        generate(&body);
    }

    #[test]
    fn simulate_prints_with_the_device_name() {
        let program = Program {
            instructions: vec![Instruction::Print(1), Instruction::Print(2)],
        };
        assert_eq!(simulate_to_string(&program), "sim: 1\nsim: 2\n");
    }

    #[test]
    fn kernel_simulates_the_generated_program() {
        let body = vec![Statement::Print(1)];
        let kernel = SimTarget
            .compile(&body)
            .expect("sim should compile print statements");
        let mut output = Vec::new();
        kernel.call(&mut output);
        assert_eq!(output, b"sim: 1\n");
    }
}
