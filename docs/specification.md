# Language specification

This document specifies the language as it exists today: its syntax and its meaning.
`design.md` describes the language that qlc aims for.

## Programs

A program is one source file. It is a list of functions.

A program is either **rejected** or **run**:

- A program that breaks one of the rules of this document is rejected.
  It does not run, and it prints nothing.
- Otherwise the program runs: the body of its `main` function is executed.

All rules are checked before the program runs, so a program that runs cannot fail.

## Lexical structure

The source text is a sequence of tokens, separated by optional whitespace.
Whitespace is the ASCII space, tab, line feed, form feed and carriage return.
There are no comments.

| Token       | Form                                                                 |
|-------------|----------------------------------------------------------------------|
| keyword     | `fn`, `let`, `print`, `delete`, `sim_device`, `thread`, `join`       |
| identifier  | an ASCII letter or `_`, followed by ASCII letters, digits or `_`; not a keyword |
| integer     | one or more ASCII digits, in base 10                                 |
| punctuation | `(`, `)`, `{`, `}`, `[`, `]`, `;`, `=`, `,`, `@`                     |

- Each token is as long as possible. For example, `letx` is one identifier, not `let` followed by `x`.
- An integer must be at most 9223372036854775807 (2^63 − 1). Leading zeros are allowed.
- Any other character is an error.

## Syntax

```text
program          = function*
function         = "fn" identifier [ place_parameters ] [ places ] "(" ")" block
place_parameters = "[" place { "," place } "]"
places           = place | "@" "(" identifier { "," identifier } ")"
place            = "@" identifier
block            = "{" statement* "}"
statement        = print | delete | join | let
print            = "print" "(" expression ")" ";"
delete           = "delete" "(" expression ")" ";"
join             = "join" "(" expression ")" ";"
let              = "let" identifier "=" expression ";"
expression       = integer | identifier | sim_device | thread
sim_device       = "sim_device" "(" ")"
thread           = "thread" "(" identifier "@" identifier ")"
```

A program that does not follow this grammar is rejected.

## Functions

- A program must have exactly one function named `main`.
- Two functions cannot have the same name.
- Functions have no parameters, and there is no way to call a function.
  The only way to run another function is to create a thread for one of its parts.
  The rules of this document still apply to every function.
- Each function is checked on its own.

## Places

A place is a participant that runs code. `docs/places.md` describes the place system that qlc aims for.

```text
fn hello[@a, @b]@(a, b)() { ... }   // place parameters a and b; runs on a and b
fn helper[@a]() { ... }             // place parameter a; runs on host
fn main@host() { ... }              // runs on host; the same as `fn main()`
```

- `[@a, @b]` declares the **place parameters** of the function.
  Two place parameters of a function cannot have the same name.
- `@a` or `@(a, b)` gives the places the function **runs on**. A place cannot be listed twice.
- In the places a function runs on, a name refers to a place parameter of the function.
  Otherwise, `host` refers to the built-in place `host`, the computer that runs qlc.
  Any other name is an error.
  So a place parameter named `host` hides the built-in place.
- A function whose header does not give its places runs on the built-in place `host`.
  This is true even if it has place parameters.
- `main` cannot have place parameters. So it runs on `host` only.

Place names are separate from function and variable names.

Each value of a function is known by every place the function runs on.
Each of these places has its own copy of the value.
A value of kind Resource cannot be copied.
So a function that runs on several places cannot use a value of kind Resource.

```text
fn hello[@a, @b]@(a, b)() {
    let d = sim_device();   // rejected: a SimDevice cannot be known by both a and b
    delete(d);
}
```

### Threads

A **part** of a function is its code on one of the places it runs on.
`thread(f@a)` creates a thread that runs the part of the function `f` on its place `a`.

- `f` is a function of the program. It can be defined before or after the `thread` expression.
  Any other name is an error.
- `a` is a place of the header of `f`, not of the function that contains the expression.
  It must be one of the places `f` runs on. Otherwise the program is rejected.
- Only a function that runs on `host` only can create a thread.
- A thread cannot run the part of a function on `host`.

```text
fn work[@a]@a() {
    print(1);
}

fn main() {
    let t = thread(work@a);   // the part of `work` on `a` runs in a thread
    print(2);
    join(t);                  // waits for the thread
}
```

So `main` runs on `host`, and the code of a function that runs on other places runs
only in a thread. A function that runs on `host` other than `main` never runs.

## Variables

A `let` statement defines a variable and sets it to the value of its expression.

- An identifier in an expression refers to the most recent `let` with that name
  before it, in the same function.
  If there is no such `let`, the program is rejected.
- In `let x = e;`, the expression `e` is evaluated before `x` is defined.
  So in `let x = x;`, the `x` on the right refers to an earlier `x`.
- A `let` can reuse the name of an earlier variable. This defines a new variable,
  which shadows the earlier one. The earlier variable still exists, but its name
  now refers to the new variable.
- A variable is visible from its `let` to the end of its function.
  Variables of one function are not visible in other functions.

## Types

Each value has a type. Each type has a kind: **Data** or **Resource**.

| Type        | Kind     | Values                                                    |
|-------------|----------|-----------------------------------------------------------|
| `Int`       | Data     | signed 64-bit integers                                    |
| `SimDevice` | Resource | simulated devices, which have no operations other than creation and deletion |
| `Thread`    | Resource | threads that run one part of a function                   |

The type of each expression:

| Expression    | Type                                   |
|---------------|----------------------------------------|
| integer       | `Int`                                  |
| `sim_device()`| `SimDevice`                            |
| `thread(f@a)` | `Thread`                               |
| identifier    | the type of the variable it refers to  |

The type of a variable is the type of the expression of its `let`.

The statements accept these types. Any other type is an error.

| Statement   | Type of the expression |
|-------------|------------------------|
| `print(e);` | `Int`                  |
| `delete(e);`| `SimDevice`            |
| `join(e);`  | `Thread`               |
| `let x = e;`| any type               |

## Resources

A value of kind Resource is used exactly once.

- Reading a variable whose type is of kind Resource **moves** its value.
  For example, `delete(d);` and `let e = d;` both move the value of `d`,
  and `join(t);` moves the value of `t`.
- A variable cannot be read after its value is moved.
- The value of each variable whose type is of kind Resource must be moved
  before the end of its function.

Values of kind Data can be read any number of times, or never.

Example of a valid function:

```text
fn main() {
    let d = sim_device();
    let e = d;      // moves the value of d into e
    delete(e);      // moves the value of e
}
```

Examples of rejected functions:

```text
fn main() {
    let d = sim_device();
    delete(d);
    delete(d);      // d is read after its value is moved
}

fn main() {
    let d = sim_device();
}                   // the value of d is never moved
```

## Execution

The statements of `main` are executed one after the other, in order.

| Statement     | Effect                                                              |
|---------------|---------------------------------------------------------------------|
| `let x = e;`  | evaluates `e` and sets `x` to its value                             |
| `print(e);`   | evaluates `e` and writes its value in base 10, followed by a line feed, to the output |
| `delete(e);`  | evaluates `e` and deletes the simulated device                      |
| `join(e);`    | evaluates `e` and waits for the thread                              |

| Expression     | Value                                     |
|----------------|-------------------------------------------|
| integer        | the integer                               |
| `sim_device()` | a new simulated device                    |
| `thread(f@a)`  | a new thread that runs the part of `f` on `a` |
| identifier     | the current value of the variable         |

Creating and deleting a simulated device have no visible effect.

What a thread prints is written to the output when the thread is joined,
so the output of a program does not depend on when its threads run.
