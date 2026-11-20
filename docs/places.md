# Place system: specification (draft)

Status: design draft, 2026-11-20.

This document specifies where values are known, where code runs, and how one
multi-device function is split into per-device parts. Timeline constructs
(`wait`, durations, parallel branches) are specified elsewhere and appear here
only where they interact with places.

Items marked **(open)** are proposals that have not been settled; they are
collected in section 13.

## 1. Concepts

| Term | Meaning |
|---|---|
| Place | A static name for a participant, written `@name`. It is a role, not a physical device. |
| `host` | Built-in place. Top-level code with no place annotation belongs to it. |
| Device type | User-defined interface (`Pulser`, `Counter`) listing the operations a device offers. Used as a bound on a place and as the type of a host handle. |
| Handle | Ordinary host value of a device type, referring to physical hardware. Never written with `@`. |
| Context | Inside a body, the places named by the nearest enclosing `@`: the function header or a `@x { }` block. In a host function it is `{host}`. |
| Part | The projection of a function onto one of its places. |

Two syntactic families never mix:

- `[...]` declares and passes generic parameters (places and types).
- `@` marks everything that concerns places.

## 2. Syntax

```
place         = "@" ident
shared        = place | "@{" ident { "," ident } "}"
distributed   = place | "@(" ident { "," ident } ")"

type          = path [ generic_args ] [ shared ]
              | "fn" [ distributed ] "(" [ types ] ")" [ "->" type ]

generic_param = place [ ":" bounds ] | ident [ ":" bounds ]
generic_arg   = place | type

fn_decl       = [ distributed ] "fn" ident [ generic_params ] [ distributed ]
                "(" [ params ] ")" [ "->" type ] [ where_clause ] block
narrow_block  = distributed block
device_op     = place "." ident "(" [ args ] ")"
projection    = expr "@" ident
where_clause  = "where" link_bound { "," link_bound }
link_bound    = ident ":" "Link" "[" place "," place "]"
```

- `@{a, b}` is a set: the same thing at every listed place.
- `@(a, b)` is an ordered tuple: one thing with a different part at each place.
- With one place the two coincide: `@a`.
- In a generic list, places come before type parameters.
- A `fn_decl` uses at most one of its two `distributed` slots. The prefix slot
  (decorator style, on the line above or the same line) is allowed only when
  all its places are already in scope. The slot after the generic list is
  always allowed.

```
@det
fn atom(t: Time) -> Bool { ... }            // prefix slot

fn bright[@p: Arith]@p(n: Int) -> Bool      // slot after generics
```

## 3. Values

| Form | Meaning |
|---|---|
| `T@a` | Local: known on `a` only. |
| `T@{a, b}` | Shared: one value, with an identical copy on each place. |

- A leaf type (`Int`, `Bool`, `Time`) is local or shared. `Int@(a, b)` is not a type.
- A shared value coerces to any non-empty subset of its places. Nothing coerces
  to a larger set; only a crossing (section 7) enlarges it.

## 4. Structs

A struct definition is one of two kinds. Mixing the two is an error.

**Place-free.** No field has a place. The struct behaves like a leaf type and
receives its place where it is used.

```
struct Point { x: Float, y: Float }
// uses: Point@b, Point@{a, b}
```

**Placed.** Every field has a place (a field whose type is already placed
counts). The places come from the struct's own generic list or from an
enclosing generic module.

```
struct Pair[@p, @q, T] {
  left:  T@p,
  right: T@q,
}
// use: Pair[@a, @b, Int]
```

- Place arguments of a placed struct are always written in full. There is no
  positional elision. Use a type alias to avoid repetition:
  `type Counts = Pair[@seq, @det, Int];`
- For any type, `knows(T)` is the intersection of the places of its leaves and
  `spans(T)` is their union. A value can be computed with or branched on only
  by places in `knows`.
- `S@p` on a placed struct is its projection: the struct restricted to the
  fields known on `p`.

## 5. Functions

### 5.1 Where a function runs

```
fn main() { ... }                         // host
fn atom@det(t: Time) -> Bool { ... }      // one place
fn shot@(seq, det)(max: Int) -> Shot      // distributed over two places
```

- A function runs where its `@` says.
- A function with no `@` runs on the host. Nothing is inherited from a module
  or from the function's generic parameters.
- A function may be generic in a place without running on it.

### 5.2 Function types

```
fn@det(Time) -> Bool
fn@(a, c)(Int) -> Shot
```

The place after `fn` says where the function runs. A postfix place on the whole
type says where the function value is known:
`(fn@det(Time) -> Bool)@{seq, det}`.

### 5.3 Context and defaults

Inside a body, the context is the set of places on the nearest enclosing `@`:

| Position | Context |
|---|---|
| Body of a host function | `{host}` |
| Body of `fn f@(a, b)` | `{a, b}` |
| Inside `@b { ... }` within that body | `{b}` (must be a subset of the enclosing context) |

Defaults, all readable from the function's own header:

1. An unannotated leaf or place-free struct type is known by every place of the
   context: `x: Int` in `fn f@(a, b)` means `x: Int@{a, b}`.
2. A bare function type `fn(...)` means a function running on the context.
3. Local bindings take their places from the right-hand side.
4. Signatures are never inferred from bodies.

## 6. Generics over places

### 6.1 Place parameters

```
struct Pair[@p, @q, T] { ... }
fn ping[@p: Pulser, @q: Pulser]@(p, q)(n: Int) where l: Link[@p, @q]
```

A bound on a place is a device type or capability interface. A bound is
optional; the sigil alone marks the parameter as a place.

### 6.2 Generic modules

```
mod tweezer[@seq: Pulser, @det: Counter]
  where l: Link[@det, @seq]
{
  struct Shot { tries: Int@seq, counts: Int@det }

  @det
  fn atom(t: Time) -> Bool { ... }

  @(seq, det)
  fn shot(max: Int) -> Shot { ... }
}
```

- The module introduces places once for the items inside it. Places are thus
  declared before any signature that uses them, and the prefix `@` slot can
  refer to them.
- A module has no places of its own. Being generic in `@seq` says nothing about
  where its functions run; each function states that itself.
- Items inside refer to each other by name with no place arguments.
- From outside, arguments go on the module: `tweezer[@a, @c].shot`.
- Semantics **(open)**: sugar. The header's parameters are added to each item
  and passed between siblings automatically, so the compiler has a single
  generics mechanism.

### 6.3 Device types and capabilities

```
interface Pulser: Branch {
  fn pulse(ch: Chan, t: Time);
}
interface Counter: Branch {
  fn count(t: Time) -> Int;
}
```

- A device operation is written with the place as receiver: `@seq.pulse(mot, 5ms)`
  **(open)**. It is looked up in the bound of that place.
- Capabilities are interfaces (`Branch`, `Arith`). Run-time control flow
  requires `Branch` on every place that acts inside it.

## 7. Links and crossings

- A link is declared in a `where` clause: `where l: Link[@det, @seq]` **(open)**.
- A crossing is a call of the link: `l(x)`. It is the only way knowledge moves
  between places.
- `l(x)` requires `det` to know `x`, and returns `x` known additionally on `seq`.
- A crossing takes the link's latency, a constant of the link type, so time
  stays known to both ends.
- Crossings are never inserted implicitly.

## 8. Place rules

`K(e)` is the set of places that know `e`. `C` is the current context.

| # | Construct | Rule |
|---|---|---|
| 1 | Literal, constant | `K = C` |
| 2 | Device op `@d.op(args)` | `d ∈ C`; `d ∈ K(arg)` for each argument; result `K = {d}` |
| 3 | Built-in operator | `K` = intersection of the operands' `K`; empty is an error. Each place computes its own copy. |
| 4 | Crossing `l(x)`, `l: Link[@s, @t]` | requires `s ∈ K(x)`; result `K(x) ∪ {t}` |
| 5 | `let x = e` | `K(x) = K(e)`; an annotation may shrink it |
| 6 | Assignment `x = e` | requires `K(e) ⊇ K(x)` |
| 7 | `if c`, `while c`, `match c` | every place that acts in the body must be in `K(c)` and satisfy `Branch` |
| 8 | Field access `s.f` | `K` = place of the field |
| 9 | Call `g(args)` | the places `g` runs on must be a subset of `C` and pairwise distinct; each argument must be known wherever its parameter requires; bounds and link constraints of `g` must hold |
| 10 | Narrowing `@x { }` | `x ⊆ C`; inside, the context is `x`; other places rest for the block's duration |

A place *acts* in a region if the region contains a device operation on it, a
crossing that starts or ends on it, or a call to a function that runs on it.

**Time.** The current time `now` is a value with its own `K`.

- A statement of statically known duration leaves `K(now)` unchanged.
- After a branch or loop on `c`, `K(now)` becomes `K(now) ∩ K(c)`, unless all
  arms have the same static duration.
- A place may act only if it is in `K(now)`.
- `sync` is the crossing for time: it restores a shared `now`.

**Calls across contexts.**

- Device code cannot call a host function.
- Host code cannot call a device function. It can only take its parts
  (section 9) and hand them to glue (section 10).
- A function generic in the single place it runs on (for example
  `util[@p].bright`) gets `@p` inferred from the places of its arguments. If the
  arguments are shared on a set `S`, each place in `S` runs its own instance and
  the result is known on `S`.

## 9. Projection

```
shot@det          // named function
let f = shot;
f@det             // binding
ping[@a, @b]@a    // after instantiation
```

- `f@p` is the part of `f` that runs on `p`. `p` must be one of the places `f`
  runs on.
- A distributed function value carries one compiled part per place, so
  projection of a binding is a field selection and works for functions chosen
  at run time. Projection itself happens at compilation.
- Type of a part: `shot@det : fn@det(max: Int) -> Shot@det`.
  - Shared parameters stay, known on `det` only.
  - Parameters known only elsewhere disappear.
  - The result is restricted to `det` (section 4).
  - Waits are kept, so parts stay aligned in time.
- A part contains one end of each crossing. It cannot be called from device
  code; parts are values for the host.

## 10. Host side

- A handle is an ordinary variable of a device type. It never carries `@`.
- The same device type bounds a place (`@a: Pulser`) and types a handle
  (`ha: Pulser`).
- Places are never concrete. Which hardware plays a role is decided at run time,
  by loading a part onto a handle.
- `run` is a library function, not an intrinsic:

```
fn run[@a: Pulser, @c: Counter](
  f:  fn@(a, c)(Int) -> Shot,
  ha: Pulser,
  hc: Counter,
  n:  Int,
) {
  ha.load(f@a);
  hc.load(f@c);
  start(ha, hc, n);
}
```

**Checked:** `load` accepts a part only if its place has the handle's device
type, so `hc.load(f@a)` is a type error.

**Not checked:** which of two same-typed roles a handle receives, and whether
every part is loaded exactly once before `start`. Both are confined to glue.

On the host, `@` appears only in generic lists, in types, and in projections.
Link constraints cannot be verified against handles statically; the glue checks
or establishes the connection at run time.

## 11. Compiler passes

1. Name resolution.
2. Type checking, with places ignored (`Int@a` and `Int@b` are both `Int`).
3. Place pass: apply defaults, infer `K` and place arguments, check section 8.
4. Projection.
5. Lowering per device type.

Resolution never depends on the enclosing places. The one place-related lookup
is a device operation, which uses the bound of the place written in the
expression. Resolution may read declared places, never inferred ones. There is
no overloading by place.

## 12. Example

```
mod tweezer[@seq: Pulser, @det: Counter]
  where l: Link[@det, @seq]
{
  struct Shot {
    tries:  Int@seq,
    counts: Int@det,
  }

  @det
  fn atom(t: Time) -> Bool {
    @det.count(t) > 12
  }

  @(seq, det)
  fn shot(max: Int) -> Shot {
    let mut tries = 0;             // Int@{seq, det}
    let mut have = false;          // Bool@{seq, det}

    while !have && tries < max {   // condition known by both
      @seq.pulse(mot, 5ms);        // det rests
      let ok = atom(20us);         // Bool@det, seq rests
      have = l(ok);                // Bool@{seq, det}
      tries = tries + 1;
    }

    @seq.pulse(probe_beam, 10us);
    let c = @det.count(50us);      // Int@det
    Shot { tries: tries, counts: c }
  }
}

fn main() {
  let ha = open_pulser("pulser0");
  let hc = open_counter("counter0");
  run(tweezer.shot, ha, hc, 5);
}
```

Rejected by the place pass:

```
let n = @det.count(20us);
if n > 12 { @seq.pulse(mot, 1ms); }
// rule 7: seq acts in a branch on `n`, which only det knows

let s = shot(5);
s.tries + s.counts
// rule 3: no place knows both operands
```

In the loop, the iteration count is only known at run time, yet `now` stays
shared because the condition is shared and the crossing has a fixed latency.

## 13. Open points

1. **Device-operation receiver.** `@seq.pulse(...)` versus a bare name in
   receiver position.
2. **Link spelling.** `where l: Link[@s, @t]` with a named link; whether there is
   at most one link per ordered pair of places; latency as a constant of the
   link type.
3. **Generic module semantics.** Sugar over item generics (proposed) versus a
   module that alone is generic; spelling of a named instance
   (`mod p = tweezer[@a, @c];`).
4. **Roles at the outermost host call.** In `run(tweezer.shot, ha, hc, 5)` the
   places of `run` and of `tweezer` are unified but never bound to anything
   concrete. They are treated as fresh roles; how link constraints on them are
   discharged at run time needs a precise rule.
5. **Results on the host.** The type under which a placed result such as `Shot`
   is gathered and returned to host code.
6. **A general `run`.** One `run` for any number of devices and any result type
   needs variadic place lists and a type parameter that takes places.
7. **Kinds of type parameters.** Whether `T` in `Pair[@p, @q, T]` is place-free
   (used as `T@p`) or placed (used bare), and how that is declared.
8. **Shared function types.** Whether `fn@{a, b}` is needed as a written type.
9. **Implicit receiver.** A bare `count(t)` inside `fn atom@det` standing for
   `@det.count(t)`. Deferred; compatible with section 11 since the header is
   syntactic.

## 14. Rejected alternatives

- `on (...)` headers and blocks as the carrier of places; replaced by `[...]`
  for declaration and `@` for where code runs.
- Devices as leading call arguments, and the receiver form `(a, b).shot()`.
- Positional binding of places, at calls or in types.
- A default where a function runs on the place parameters it declares.
- Groups of functions inheriting a place from an enclosing `@ { }`.
- A trailing clause `fn atom() -> R on det`.
- `Handle[@a]` as the handle type, and static `place` declarations.
- Resolution that depends on the enclosing places.