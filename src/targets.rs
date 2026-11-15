//! Targets are the devices that run `on` blocks, such as a sequencer or an FPGA.
//!
//! A target works in two steps:
//! - At compile time, [`Target::compile`] turns the body of an `on` block into a [`Kernel`].
//! - At runtime, the interpreter calls [`Kernel::call`] each time the block runs.
//!
//! The rest of qlc only knows the [`Target`] and [`Kernel`] traits and the [`Registry`].
//! Each target has its own module, with its own IR, code generation and execution.

pub mod sim;

use std::collections::HashMap;
use std::fmt;
use std::io::Write;

use crate::ast::Statement;

/// A device that can run the body of an `on` block.
pub trait Target {
    /// Compiles `body` for this device.
    /// Returns an error if the device cannot run `body`.
    fn compile(&self, body: &[Statement]) -> Result<Box<dyn Kernel>, TargetError>;
}

/// An `on` block compiled by a target.
pub trait Kernel {
    /// Runs the block on the device. What the device prints is written to `output`.
    fn call(&self, output: &mut dyn Write);
}

/// An error found by a target when it compiles an `on` block.
#[derive(Debug, PartialEq, Eq)]
pub struct TargetError(pub String);

impl fmt::Display for TargetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The targets that `on` blocks can use, by name.
pub struct Registry {
    targets: HashMap<String, Box<dyn Target>>,
}

impl Registry {
    /// A registry without targets.
    pub fn empty() -> Self {
        Registry {
            targets: HashMap::new(),
        }
    }

    /// The registry with all the targets built into qlc.
    pub fn builtin() -> Self {
        let mut registry = Registry::empty();
        registry.add("sim", Box::new(sim::SimTarget));
        registry
    }

    /// Adds `target` under `name`. The name must not be used yet.
    pub fn add(&mut self, name: &str, target: Box<dyn Target>) {
        let previous = self.targets.insert(name.to_string(), target);
        assert!(
            previous.is_none(),
            "target `{name}` should be registered only once"
        );
    }

    /// Returns the target named `name`, or `None` if there is no such target.
    pub fn get(&self, name: &str) -> Option<&dyn Target> {
        self.targets.get(name).map(|target| target.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A target that always fails to compile.
    struct FailingTarget;

    impl Target for FailingTarget {
        fn compile(&self, _body: &[Statement]) -> Result<Box<dyn Kernel>, TargetError> {
            Err(TargetError("failing cannot compile".to_string()))
        }
    }

    #[test]
    fn get_returns_an_added_target() {
        let mut registry = Registry::empty();
        registry.add("failing", Box::new(FailingTarget));
        assert!(registry.get("failing").is_some());
    }

    #[test]
    fn get_returns_none_for_an_unknown_target() {
        assert!(Registry::empty().get("unknown").is_none());
    }

    #[test]
    fn builtin_contains_sim() {
        assert!(Registry::builtin().get("sim").is_some());
    }

    #[test]
    #[should_panic(expected = "target `failing` should be registered only once")]
    fn adding_the_same_name_twice_panics() {
        let mut registry = Registry::empty();
        registry.add("failing", Box::new(FailingTarget));
        registry.add("failing", Box::new(FailingTarget));
    }
}
