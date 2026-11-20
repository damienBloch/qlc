//! The interpreter is the last stage. It executes a program from its parts IR.
//!
//! The previous stages check the program, so running it cannot fail.
//!
//! Each thread of the program runs its part on a thread of the computer.
//! What a part prints is kept until its thread is joined,
//! so the output does not depend on when the threads run.

use std::io::Write;
use std::thread::{self, Scope, ScopedJoinHandle};

use crate::parts_ir::{Expression, Part, Place, Program, Statement};

/// Runs the host part of the `main` function of `program`.
/// What the program prints is written to `output`.
pub fn run(program: &Program, output: &mut impl Write) {
    let main = program.function(program.main());
    // The scope ends when `main` ends, so every thread of the program is finished by then.
    thread::scope(|scope| {
        run_part(
            main.part(Place::Host),
            program,
            scope,
            &mut Threads::default(),
            output,
        );
    });
}

/// Runs `part`. `scope` is where its threads run,
/// and `threads` contains the threads that are not joined yet.
fn run_part<'scope, 'env>(
    part: &'env Part,
    program: &'env Program,
    scope: &'scope Scope<'scope, 'env>,
    threads: &mut Threads<'scope>,
    output: &mut dyn Write,
) {
    // The value of each variable, indexed by its id. `None` until the variable is set.
    let mut variables = vec![None; part.variable_count()];
    for statement in part.body() {
        match statement {
            Statement::Print(expression) => {
                match evaluate(expression, &variables, program, scope, threads) {
                    Value::Int(value) => writeln!(output, "{value}")
                        .expect("writing the program output should succeed"),
                    _ => panic!("the type checker should ensure that `print` gets an `Int`"),
                }
            }
            Statement::Delete(expression) => {
                match evaluate(expression, &variables, program, scope, threads) {
                    Value::SimDevice => {}
                    _ => panic!("the type checker should ensure that `delete` gets a `SimDevice`"),
                }
            }
            Statement::Join(expression) => {
                match evaluate(expression, &variables, program, scope, threads) {
                    Value::Thread(thread) => output
                        .write_all(&threads.join(thread))
                        .expect("writing the program output should succeed"),
                    _ => panic!("the type checker should ensure that `join` gets a `Thread`"),
                }
            }
            Statement::Let { variable, value } => {
                let value = evaluate(value, &variables, program, scope, threads);
                *variables.get_mut(variable.0).expect(VARIABLE_ID_IN_RANGE) = Some(value);
            }
        }
    }
}

/// Returns the value of `expression`.
/// `variables` contains the value of each variable of its part, or `None` if it is not set.
/// `scope` is where the threads of the part run,
/// and `threads` contains the threads that are not joined yet.
fn evaluate<'scope, 'env>(
    expression: &Expression,
    variables: &[Option<Value>],
    program: &'env Program,
    scope: &'scope Scope<'scope, 'env>,
    threads: &mut Threads<'scope>,
) -> Value {
    match expression {
        Expression::Integer(value) => Value::Int(*value),
        Expression::NewSimDevice => Value::SimDevice,
        Expression::Variable(variable) => variables
            .get(variable.0)
            .expect(VARIABLE_ID_IN_RANGE)
            .expect("`parts_ir::Part::new` should ensure that a variable is set before it is read"),
        Expression::NewThread { function, place } => {
            let part = program.function(*function).part(*place);
            let handle = scope.spawn(|| {
                let mut part_output = Vec::new();
                // The place checker ensures that a part run by a thread creates no thread,
                // so this part starts with no thread of its own.
                run_part(
                    part,
                    program,
                    scope,
                    &mut Threads::default(),
                    &mut part_output,
                );
                part_output
            });
            Value::Thread(threads.add(handle))
        }
    }
}

/// A value computed by the program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Value {
    Int(i64),
    /// A simulated device.
    SimDevice,
    /// A thread, which is joined once.
    Thread(ThreadId),
}

/// The threads of a part that are not joined yet.
#[derive(Default)]
struct Threads<'scope> {
    /// Each thread that is not joined yet, indexed by its [`ThreadId`].
    /// A thread gives what it printed when it is joined.
    handles: Vec<Option<ScopedJoinHandle<'scope, Vec<u8>>>>,
}

impl<'scope> Threads<'scope> {
    /// Adds the thread `handle` and returns its id.
    fn add(&mut self, handle: ScopedJoinHandle<'scope, Vec<u8>>) -> ThreadId {
        self.handles.push(Some(handle));
        ThreadId(self.handles.len() - 1)
    }

    /// Waits for the thread `thread` and returns what it printed.
    fn join(&mut self, thread: ThreadId) -> Vec<u8> {
        let handle = self
            .handles
            .get_mut(thread.0)
            .expect("a thread id should refer to a thread of the part")
            .take()
            .expect("the linearity checker should ensure that a thread is joined once");
        // A part only panics if qlc has a bug.
        // The panic is raised again here, so that the host reports it.
        handle
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
    }
}

/// The index of a thread in the threads of a program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ThreadId(usize);

const VARIABLE_ID_IN_RANGE: &str = "`parts_ir::Part::new` checks variable ids";

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::parts_ir::{Function, FunctionId, Place, PlaceParameterId, VariableId};

    /// Runs `program` and returns what it printed.
    fn run_to_string(program: &Program) -> String {
        let mut output = Vec::new();
        run(program, &mut output);
        String::from_utf8(output).expect("the output should be UTF-8")
    }

    /// A function named `name` with one host part, whose body is `body` and which has `variable_count` variables.
    fn host_function(name: &str, body: Vec<Statement>, variable_count: usize) -> Function {
        let part = Part::new(body, variable_count);
        Function::new(name.to_string(), HashMap::from([(Place::Host, part)]))
    }

    /// A program with only a `main` function, as in [`host_function`].
    fn host_program(body: Vec<Statement>, variable_count: usize) -> Program {
        Program::new(
            vec![host_function("main", body, variable_count)],
            FunctionId(0),
        )
    }

    /// A statement that prints the integer `value`.
    fn print(value: i64) -> Statement {
        Statement::Print(Expression::Integer(value))
    }

    /// A statement that prints `variable`.
    fn print_variable(variable: usize) -> Statement {
        Statement::Print(Expression::Variable(VariableId(variable)))
    }

    /// A statement that sets `variable` to the value of `value`.
    fn let_statement(variable: usize, value: Expression) -> Statement {
        Statement::Let {
            variable: VariableId(variable),
            value,
        }
    }

    /// A function named `work` with one part on the place `a`, whose body is `body`.
    fn work_function(body: Vec<Statement>) -> Function {
        let part = Part::new(body, 0);
        Function::new("work".to_string(), HashMap::from([(WORK_PLACE, part)]))
    }

    /// The place of the part of the `work` function of [`work_function`].
    const WORK_PLACE: Place = Place::Parameter(PlaceParameterId(0));

    /// An expression that creates a thread for the part of the `work` function.
    fn new_thread() -> Expression {
        Expression::NewThread {
            function: FunctionId(1),
            place: WORK_PLACE,
        }
    }

    /// A statement that joins `variable`.
    fn join_variable(variable: usize) -> Statement {
        Statement::Join(Expression::Variable(VariableId(variable)))
    }

    /// A program whose `main` function has the body `body` and `variable_count` variables,
    /// and whose `work` function prints 1 on its only place.
    fn program_with_work(body: Vec<Statement>, variable_count: usize) -> Program {
        Program::new(
            vec![
                host_function("main", body, variable_count),
                work_function(vec![print(1)]),
            ],
            FunctionId(0),
        )
    }

    #[test]
    fn thread_prints_when_it_is_joined() {
        let body = vec![
            let_statement(0, new_thread()),
            print(2),
            join_variable(0),
            print(3),
        ];
        assert_eq!(run_to_string(&program_with_work(body, 1)), "2\n1\n3\n");
    }

    #[test]
    fn each_thread_prints_once() {
        let body = vec![
            let_statement(0, new_thread()),
            let_statement(1, new_thread()),
            join_variable(0),
            join_variable(1),
        ];
        assert_eq!(run_to_string(&program_with_work(body, 2)), "1\n1\n");
    }

    #[test]
    fn runs_empty_part() {
        assert_eq!(run_to_string(&host_program(vec![], 0)), "");
    }

    #[test]
    fn runs_only_the_main_function() {
        let program = Program::new(
            vec![
                host_function("helper", vec![print(1)], 0),
                host_function("main", vec![print(2)], 0),
            ],
            FunctionId(1),
        );
        assert_eq!(run_to_string(&program), "2\n");
    }

    #[test]
    fn print_writes_the_value_on_its_own_line() {
        assert_eq!(run_to_string(&host_program(vec![print(42)], 0)), "42\n");
    }

    #[test]
    fn statements_run_in_order() {
        let body = vec![print(1), print(i64::MAX), print(0)];
        assert_eq!(
            run_to_string(&host_program(body, 0)),
            "1\n9223372036854775807\n0\n"
        );
    }

    #[test]
    fn print_writes_the_value_of_a_variable() {
        let body = vec![let_statement(0, Expression::Integer(42)), print_variable(0)];
        assert_eq!(run_to_string(&host_program(body, 1)), "42\n");
    }

    #[test]
    fn let_can_copy_another_variable() {
        let body = vec![
            let_statement(0, Expression::Integer(1)),
            let_statement(1, Expression::Variable(VariableId(0))),
            print_variable(0),
            print_variable(1),
        ];
        assert_eq!(run_to_string(&host_program(body, 2)), "1\n1\n");
    }

    #[test]
    fn new_sim_device_can_be_stored_in_a_variable() {
        let body = vec![let_statement(0, Expression::NewSimDevice), print(1)];
        assert_eq!(run_to_string(&host_program(body, 1)), "1\n");
    }

    #[test]
    fn delete_of_a_sim_device_prints_nothing() {
        let body = vec![
            let_statement(0, Expression::NewSimDevice),
            Statement::Delete(Expression::Variable(VariableId(0))),
        ];
        assert_eq!(run_to_string(&host_program(body, 1)), "");
    }
}
