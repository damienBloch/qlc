# Architecture

qlc reads one source file and executes it on the host.
Parts of the program, the `on` blocks, run on other devices, called targets.
The work is split in stages.
Each stage takes the output of the previous stage as input and produces a new representation.

## Stages

```mermaid
flowchart LR
    source["Source text<br/>(String)"]
    tokens["Tokens<br/>(Vec&lt;Token&gt;)"]
    ast["AST<br/>(ast::Program)"]
    hir["Host IR<br/>(host_ir::Program)"]
    result["Result<br/>(exit code)"]

    source -- "lexer::tokenize" --> tokens
    tokens -- "parser::parse" --> ast
    ast -- "outliner::outline" --> hir
    hir -- "interpreter::run" --> result
    ast -. "body of on blocks" .-> targets["Targets<br/>(see below)"]
    targets -. "kernels" .-> hir
```

| Stage       | File                 | Input                | Output              |
|-------------|----------------------|----------------------|---------------------|
| Lexer       | `src/lexer.rs`       | source text          | `Vec<Token>`        |
| Parser      | `src/parser.rs`      | `&[Token]`           | `ast::Program`      |
| Outliner    | `src/outliner.rs`    | `&ast::Program`      | `host_ir::Program`  |
| Interpreter | `src/interpreter.rs` | `&host_ir::Program`  | `()`                |

The outliner also takes the registry of targets.
It takes each `on` block out of its function and sends its body to its target,
which compiles it to a kernel. The `on` block is replaced by a call to this kernel.
Compilers call this "outlining".

The interpreter also takes an output writer (`&mut impl Write`).
`print` and the kernels write to it.
The command line passes stdout, and the unit tests pass a `Vec<u8>` to check the output.

The AST types are in `src/ast.rs`. They are shared by the parser, the outliner and the targets.
The host IR types are in `src/host_ir.rs`. They are shared by the outliner and the interpreter.
The host IR is the AST where each `on` block is replaced by a call to its kernel.

Each stage is one public function.
A stage only depends on the types of its input and output.
So the algorithm of a stage can be changed without changing the other stages.

## Targets

A target is a device that runs the body of `on` blocks, such as a sequencer, an FPGA or an ARTIQ kernel.
Today the only target is `sim`, a simulated device for tests.

A target works in two steps:

1. **At compile time**, the outliner calls `Target::compile` with the body of an `on` block.
   The target returns a `Kernel`, or a `TargetError` if the device cannot run the body.
2. **At runtime**, the interpreter calls `Kernel::call` each time the block runs.

```mermaid
flowchart LR
    outliner["outliner"]
    interpreter["interpreter"]
    registry["Registry<br/>name -> target"]
    target["Target trait"]
    kernel["Kernel trait"]
    subgraph sim["src/targets/sim.rs"]
        generate["sim::generate"]
        ir["sim::Program<br/>(sim IR)"]
        simulate["sim::simulate"]
        generate --> ir --> simulate
    end

    outliner -- "looks up the name" --> registry
    registry -- "contains" --> target
    outliner -- "Target::compile(body)" --> target
    target -- "returns" --> kernel
    interpreter -- "Kernel::call()" --> kernel
    sim -- "implements" --> target
    sim -- "implements" --> kernel
```

| Item          | File                 | Role                                                  |
|---------------|----------------------|-------------------------------------------------------|
| `Target`      | `src/targets.rs`     | Compiles the body of an `on` block to a kernel.       |
| `Kernel`      | `src/targets.rs`     | Runs a compiled `on` block on the device.             |
| `TargetError` | `src/targets.rs`     | An error found by a target at compile time.           |
| `Registry`    | `src/targets.rs`     | Maps target names to targets.                         |
| `sim`         | `src/targets/sim.rs` | A simulated device with its own IR.                   |

Rules:

- The rest of qlc only knows the `Target` and `Kernel` traits and the `Registry`.
  It never uses the IR of a target.
- Each target owns its IR, its code generation and its execution.
  To add a device, add a module in `src/targets/` and one line in `Registry::builtin`.
- The registry is a parameter of `outliner::outline`, not a global.
  `lib::run_source` passes `Registry::builtin()`, and the outliner tests pass a registry with fake targets.
- The interpreter does not use the registry. It only calls the kernels stored in the host IR.
- A target receives the AST of the block body. This may change to a shared device IR when there are several targets.

`sim` works in two steps, which can be tested separately:
`sim::generate` generates a `sim::Program`, and `sim::simulate` runs it.
Its kernel stores the generated program and simulates it at each call.
`sim::Program` can be printed as text, one instruction per line, to check the generated code.
Each line that `sim` prints starts with `sim: `.

Not supported yet:

- An `on` block inside another `on` block (a device that runs code on another device).
- Values passed from the host to a kernel when it is called.

## Errors

Each stage has its own error type: `LexError`, `ParseError`, `OutlineError` and `RunError`.
`src/lib.rs` wraps them in one `Error` type.

There are two kinds of problems:

- **Errors in the user's program.** A stage returns an `Err`.
  The outliner and the interpreter check the program before running it,
  so an error is reported before anything is printed.
  The command line prints `error: <message>` on stderr and exits with code 1.
- **Bugs in qlc.** The code panics, with `expect` and a reason.
  Features that are not decided yet use `todo!`.

## Command line

```mermaid
flowchart TD
    args["qlc &lt;file&gt;"] --> read["Read the file"]
    read --> run["lib::run_source<br/>(all stages)"]
    run -- "Ok" --> ok["exit code 0"]
    run -- "Err" --> err["print error, exit code 1"]
    read -- "cannot read" --> err
```

`src/main.rs` only reads the file and reports the result.
All the language logic is in the library (`src/lib.rs` and the stage modules).

## Language

The language currently supports functions without parameters.
A function body is a list of statements:

```text
program   = function*
function  = "fn" identifier "(" ")" block
block     = "{" statement* "}"
statement = print | on
print     = "print" "(" integer ")" ";"
on        = "on" identifier block
```

An integer is a sequence of ASCII digits whose value fits in an `i64`.
`print` writes the integer on its own line on stdout.

A program must contain exactly one function named `main`.
Function names must be unique.
Running a program calls `main`.

`on <target> { ... }` runs its body on the target named `<target>`.
The target must exist, and must accept the body, even if the block is in a function that is never called.

## Tests

- **Unit tests** are in each stage module. They test the stage function directly.
  The outliner tests use fake targets, and the interpreter tests use fake kernels,
  so they do not depend on `sim`.
- **Integration tests** are in `tests/run_programs.rs`.
  They run the `qlc` binary on the files in `tests/programs/` and check only the exit code, stdout and stderr.
