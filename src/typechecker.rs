//! The type checker is the fourth stage. It checks the types of the resolved AST
//! and returns the typed AST.
//!
//! The type of a variable is the type of the value of its `let`.
//! Today `Int` is the only type, so no type error can occur.

use crate::resolved_ast;
use crate::typed_ast;
use crate::types::Type;

/// Checks the types of `program` and returns its typed AST.
pub fn check(program: resolved_ast::Program) -> typed_ast::Program {
    let (functions, main) = program.into_parts();
    let functions = functions.into_iter().map(check_function).collect();
    typed_ast::Program::new(functions, main)
}

fn check_function(function: resolved_ast::Function) -> typed_ast::Function {
    let mut variable_types = VariableTypes::new(function.variable_count);
    let body = function
        .body
        .into_iter()
        .map(|statement| check_statement(statement, &mut variable_types))
        .collect();
    typed_ast::Function {
        name: function.name,
        body,
        variable_types: variable_types.into_types(),
    }
}

/// Checks `statement`. `variable_types` contains the types of the variables set before it.
fn check_statement(
    statement: resolved_ast::Statement,
    variable_types: &mut VariableTypes,
) -> typed_ast::Statement {
    match statement {
        resolved_ast::Statement::Print(value) => {
            let (value, value_type) = check_expression(value, variable_types);
            match value_type {
                Type::Int => typed_ast::Statement::Print(value),
            }
        }
        resolved_ast::Statement::Let { variable, value } => {
            let (value, value_type) = check_expression(value, variable_types);
            variable_types.set(variable, value_type);
            typed_ast::Statement::Let { variable, value }
        }
    }
}

/// Checks `expression` and returns it with its type.
/// `variable_types` contains the types of the variables set before it.
fn check_expression(
    expression: resolved_ast::Expression,
    variable_types: &VariableTypes,
) -> (typed_ast::Expression, Type) {
    match expression {
        resolved_ast::Expression::Integer(value) => {
            (typed_ast::Expression::Integer(value), Type::Int)
        }
        resolved_ast::Expression::Variable(variable) => (
            typed_ast::Expression::Variable(variable),
            variable_types.get(variable),
        ),
    }
}

/// The types of the variables of a function, found so far.
struct VariableTypes {
    /// The type of each variable, indexed by its id. `None` if its `let` is not checked yet.
    types: Vec<Option<Type>>,
}

impl VariableTypes {
    /// Creates `count` variables whose types are not known yet.
    fn new(count: usize) -> Self {
        VariableTypes {
            types: vec![None; count],
        }
    }

    fn set(&mut self, variable: resolved_ast::VariableId, variable_type: Type) {
        let slot = self
            .types
            .get_mut(variable.0)
            .expect("a variable id should be smaller than the variable count of its function");
        assert!(
            slot.is_none(),
            "each variable should be set by only one `let`"
        );
        *slot = Some(variable_type);
    }

    fn get(&self, variable: resolved_ast::VariableId) -> Type {
        self.types
            .get(variable.0)
            .expect("a variable id should be smaller than the variable count of its function")
            .expect("the resolver should ensure that a variable is set before it is read")
    }

    /// Returns the type of each variable. Panics if a variable has no type.
    fn into_types(self) -> Vec<Type> {
        self.types
            .into_iter()
            .map(|variable_type| {
                variable_type.expect("the resolver should define each variable with a `let`")
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use resolved_ast::{FunctionId, VariableId};

    /// Checks a program whose only function is `main`, with the body `body`
    /// and `variable_count` variables.
    fn check_main(body: Vec<resolved_ast::Statement>, variable_count: usize) -> typed_ast::Program {
        let main = resolved_ast::Function {
            name: "main".to_string(),
            body,
            variable_count,
        };
        check(resolved_ast::Program::new(vec![main], FunctionId(0)))
    }

    /// The types of the variables of the `main` function of `program`.
    fn main_variable_types(program: &typed_ast::Program) -> &[Type] {
        &program.function(program.main()).variable_types
    }

    /// A resolved statement that sets `variable` to the integer `value`.
    fn let_integer(variable: usize, value: i64) -> resolved_ast::Statement {
        resolved_ast::Statement::Let {
            variable: VariableId(variable),
            value: resolved_ast::Expression::Integer(value),
        }
    }

    /// A resolved statement that sets `variable` to the value of `source`.
    fn let_variable(variable: usize, source: usize) -> resolved_ast::Statement {
        resolved_ast::Statement::Let {
            variable: VariableId(variable),
            value: resolved_ast::Expression::Variable(VariableId(source)),
        }
    }

    /// A resolved statement that prints `variable`.
    fn print_variable(variable: usize) -> resolved_ast::Statement {
        resolved_ast::Statement::Print(resolved_ast::Expression::Variable(VariableId(variable)))
    }

    #[test]
    fn function_bodies_are_checked() {
        let program = check_main(vec![let_integer(0, 42), print_variable(0)], 1);
        assert_eq!(
            program.function(program.main()),
            &typed_ast::Function {
                name: "main".to_string(),
                body: vec![
                    typed_ast::Statement::Let {
                        variable: VariableId(0),
                        value: typed_ast::Expression::Integer(42),
                    },
                    typed_ast::Statement::Print(typed_ast::Expression::Variable(VariableId(0))),
                ],
                variable_types: vec![Type::Int],
            }
        );
    }

    #[test]
    fn main_stays_the_same() {
        let functions = ["helper", "main"].map(|name| resolved_ast::Function {
            name: name.to_string(),
            body: vec![],
            variable_count: 0,
        });
        let program = check(resolved_ast::Program::new(functions.into(), FunctionId(1)));
        assert_eq!(program.main(), FunctionId(1));
    }

    #[test]
    fn variable_set_to_an_integer_is_an_int() {
        let program = check_main(vec![let_integer(0, 1)], 1);
        assert_eq!(main_variable_types(&program), [Type::Int]);
    }

    #[test]
    fn variable_set_to_a_variable_has_its_type() {
        let program = check_main(vec![let_integer(0, 1), let_variable(1, 0)], 2);
        assert_eq!(main_variable_types(&program), [Type::Int, Type::Int]);
    }

    #[test]
    #[should_panic(
        expected = "the resolver should ensure that a variable is set before it is read"
    )]
    fn variable_read_before_its_let_panics() {
        check_main(vec![print_variable(0), let_integer(0, 1)], 1);
    }

    #[test]
    #[should_panic(expected = "the resolver should define each variable with a `let`")]
    fn variable_without_let_panics() {
        check_main(vec![], 1);
    }

    #[test]
    #[should_panic(expected = "each variable should be set by only one `let`")]
    fn variable_set_twice_panics() {
        check_main(vec![let_integer(0, 1), let_integer(0, 2)], 1);
    }
}
