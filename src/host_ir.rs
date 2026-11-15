//! The host IR. The outliner produces it and the interpreter runs it.
//!
//! It is the AST where each `on` block is replaced by a call to a [`Kernel`],
//! compiled by the target of the block.

use crate::targets::Kernel;

pub struct Program {
    functions: Vec<Function>,
    /// The compiled `on` blocks. [`Statement::CallKernel`] refers to them by their index.
    kernels: Vec<Box<dyn Kernel>>,
}

impl Program {
    /// Creates a program. `kernels` contains the kernels called by `functions`,
    /// in the order of their [`KernelId`].
    pub fn new(functions: Vec<Function>, kernels: Vec<Box<dyn Kernel>>) -> Self {
        Program { functions, kernels }
    }

    pub fn functions(&self) -> &[Function] {
        &self.functions
    }

    /// Returns the kernel `id`, or `None` if the program has no such kernel.
    pub fn kernel(&self, id: KernelId) -> Option<&dyn Kernel> {
        self.kernels.get(id.0).map(|kernel| kernel.as_ref())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Function {
    pub name: String,
    pub body: Vec<Statement>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Statement {
    /// Prints an integer on its own line.
    Print(i64),
    /// Runs a compiled `on` block.
    CallKernel(KernelId),
}

/// The index of a kernel in [`Program::kernels`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KernelId(pub usize);
