//! Targets are the devices that run `on` blocks, such as a sequencer or an FPGA.
//!
//! The interpreter only knows the [`Target`] trait and the [`Registry`].
//! Each target has its own module, with its own IR, code generation and execution.

pub mod sim;

use std::collections::HashMap;
use std::io::Write;

use crate::ast::Statement;

/// A device that can run the body of an `on` block.
pub trait Target {
    /// Generates a program for this device from `body`, and runs it.
    /// What the device prints is written to `output`.
    fn run(&self, body: &[Statement], output: &mut dyn Write);
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

    /// A target that does nothing.
    struct NoopTarget;

    impl Target for NoopTarget {
        fn run(&self, _body: &[Statement], _output: &mut dyn Write) {}
    }

    #[test]
    fn get_returns_an_added_target() {
        let mut registry = Registry::empty();
        registry.add("noop", Box::new(NoopTarget));
        assert!(registry.get("noop").is_some());
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
    #[should_panic(expected = "target `noop` should be registered only once")]
    fn adding_the_same_name_twice_panics() {
        let mut registry = Registry::empty();
        registry.add("noop", Box::new(NoopTarget));
        registry.add("noop", Box::new(NoopTarget));
    }
}
