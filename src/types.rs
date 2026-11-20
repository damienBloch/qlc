//! The types of the values of a program.

use std::fmt;

/// The type of a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    /// A 64-bit signed integer.
    Int,
    /// A simulated device.
    SimDevice,
    /// A thread that runs a part of a function.
    Thread,
}

impl Type {
    pub fn kind(self) -> Kind {
        match self {
            Type::Int => Kind::Data,
            Type::SimDevice => Kind::Resource,
            Type::Thread => Kind::Resource,
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Int => write!(f, "Int"),
            Type::SimDevice => write!(f, "SimDevice"),
            Type::Thread => write!(f, "Thread"),
        }
    }
}

/// How the values of a type can be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A value that can be copied and dropped.
    Data,
    /// A value that must be used exactly once. It cannot be copied or dropped.
    Resource,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int_is_data() {
        assert_eq!(Type::Int.kind(), Kind::Data);
    }

    #[test]
    fn sim_device_is_a_resource() {
        assert_eq!(Type::SimDevice.kind(), Kind::Resource);
    }

    #[test]
    fn thread_is_a_resource() {
        assert_eq!(Type::Thread.kind(), Kind::Resource);
    }
}
