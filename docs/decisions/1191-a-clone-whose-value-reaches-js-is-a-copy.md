# 1191. A clone whose value reaches JS is a copy

Status: Accepted (2026-11-23, #2408, the value-level design the lead
approved). Amends [0052](0052-std-trait-impls.md): a clone no Rust code
changes either side of is still a copy where its value, or its source,
may reach JS.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. This makes JS
agree with Rust: a clone handed to JS is its own value, as it is
natively.

## Context

A clone is left out where no Rust code changes either copy (ADR 0052,
`Changed`): `players.clone()` is `players`, since nothing could tell the
two apart. JS can. A function that returns two `Match`es built from one
`Vec` and its clone gives JS one array twice, so pushing onto one match's
players adds to the other's. A clone of an element of what JS gave,
returned, hands JS back its own object. The guides pin both
(coming-from-javascript's chapters 18 and 24).

A per-type fix, every type a JS-facing function returns counted as
changed, copied far more than needed. It also changed library contracts
(ADR 0914), and two of the React port's crates stopped compiling.

## Decision

**A clone keeps its copy where its value or its source may reach JS,
found value by value (`copies/reached.rs`).**

- **Where a value reaches JS:** an exported function's result, a returned
  closure's result, what Rust gives a binding or a callback JS gave, and a
  constant read into any of those.
- **Found backwards through each body's MIR** from those places, through
  moves, aggregates and fields. Across calls it uses a summary of which
  parameters flow into a function's result, iterated to a fixpoint.
- **Where the flow can't be followed, the copy is kept:**
  - a store through a `&mut` into what reaches JS reaches it with every
    argument of that call;
  - a borrow, a raw pointer, an `Rc`, an `Arc` or a `Weak` shares its
    source's storage, so they're one alias class;
  - a library's or std's function's result carries every argument.
- **Only what can be an object counts.** A flow into a number, a string, a
  `bool`, or an `Option` of one carries nothing JS can change.
- **What a clone copies is recorded with its parts:** an `Option`'s value,
  a tuple's items, a struct's or an enum's fields, and std types'
  arguments. A copy is decided part by part, the same way.
- **A clone read only through its fields records only its parts.**
  `let (node, depth) = stack[at].clone()` never hands JS the tuple, so
  its JS destructures, `const [node, depth] = stack[at]`, as React's
  `findPaths` does. A clone used whole, moved, borrowed or returned,
  records itself too. A `let` pattern's fake read and a discriminant's
  read aren't uses of the whole value.
- **Read beside `Changed`, per function** (`js_copies_here`, by typeck
  root), in `mutated_itself`, `vec_changed` and `items_changed`.
  - A reference, an `Rc` and a function are never forced, as their clones
    share by design. An `Option` of one is the same.
  - Neither is what's never an object (`may_be_object`), nor a JS
    object a binding declares, a `Promise` (`is_js_object`), whose clone
    is the same object.
  - Nor an untagged enum: it's its payload, so its copy is its
    payload's, decided part by part. The React port's `ChunkEntry`,
    a `Promise` or `null`, is the case.
  - A recorded type parameter is only itself.
- `Changed`, library contracts and ADR 0914 are untouched: no manifest
  changes.

## Why

- **JS gets what Rust gives.** A clone is a value of its own, and JS can
  change values in place.
- **Value by value, not type by type.** Most clones never reach JS, and
  those stay left out, as before.
- **Conservative where it can't see.** A copy too many costs a spread,
  while a copy too few is a wrong answer.

## Alternatives

- **Every type a JS-facing function returns counted as changed:** too
  many copies, and contract changes that broke the port (above).
- **Per clone site:** exact, but it needs a MIR location mapped to the
  lowering of each clone call. That's filed as #2411, P3. Per function, one
  needed copy of a type makes the others in that function copy too
  (`todomvc`'s `..todo.clone()` spreads).

## Consequences

- Snapshots that change, each a value JS gets:
  - `wire`: an `order` given back inside an event;
  - `type_constant_of_a_module_constant`: a module constant returned;
  - `calc`: an input token pushed into the returned `Vec`;
  - `interpreter`: `Env::get` returns a stored value;
  - `nullable_let_read_alike`: `state.clones` cloned into `Kept`;
  - `todomvc`'s model: a todo JS gave, returned;
  - `todomvc`'s JSX: a prop given to React. That one is conservative,
    and #2410 records that React never changes props.
- The React port: `ReactFlightWebpackPlugin` copies the client references
  JS gave it.

## What holds it

- `test/js-copies.test.ts`. JS changes one value and reads the other:
  - two matches;
  - a `&mut` store and a push into a returned `Vec`;
  - a library function passing its argument on;
  - a callback JS gave;
  - and a clone nothing reaches JS through, which stays left out.
- The corpus's `clone_read_through_its_parts_copies_its_parts`: a
  destructured clone copies nothing, and the same tuple returned whole is
  a copy.
- The mutations in `scripts/mutations/lower/copies/reached.ts`.
