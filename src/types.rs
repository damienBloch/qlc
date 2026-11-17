//! The types of the values of a program.

/// The type of a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    /// A 64-bit signed integer.
    Int,
}

impl Type {
    pub fn kind(self) -> Kind {
        match self {
            Type::Int => Kind::Data,
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
}
