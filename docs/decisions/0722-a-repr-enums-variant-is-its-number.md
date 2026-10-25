# 0722. A `#[repr(..)]` enum's variant is its number

Status: Accepted. Extends [0013](0013-fieldless-enums.md) and
[0393](0393-an-enums-discriminant.md). Amended: each variant is its
module's `const` of its number, read by its name, as a `const` item is
([0031](0031-consts.md), [0770](0770-a-librarys-const-is-imported-by-its-name.md)):
`const Pending = 0;`, `payload._status = Pending` (below). Amended again:
a variant's `#[rust_js::name]` names that `const`, `const UNTERMINATED =
0;`, where it had made the enum its names (below).

Case ([0262](0262-when-rust-and-js-disagree.md)): A: which JS value a
variant is, nothing a program observes but through its own `==`, order,
`as`, `{:?}` and serde, each kept Rust's.

## Context

React's `shared/ReactPerformanceTrackProperties` keeps an array's kind
as a number, `const EMPTY_ARRAY = 0; const COMPLEX_ARRAY = 1; ..`. Its
port's `enum ArrayKind { Empty, Complex, Primitive, Entries }` printed
`'Empty'`, `'Complex'`, as every fieldless variant is its name (ADR
0013), and so did one written `#[repr(u8)] enum ArrayKind { Empty = 0,
.. }`: the Rust that says each variant's number had no way to make it
one in JS. A port had only `const EMPTY_ARRAY: u8 = 0;`, no enum.

## Decision

**A fieldless enum under a `#[repr(..)]` of an integer of 32 bits or
fewer is its numbers**, as rustc numbers its variants, written or not:
`ArrayKind::Empty` is `0`. `bindings::variant_tag` is the one answer to
what a variant is in JS, and `bindings::numbered` whether an enum's are
numbers; each question of its value asks them:

- **a test, a `match` and a `switch`** compare the number, `kind === 0`;
- **`as u8`, a `Hash` and `mem::discriminant`** are the value itself
  (`named_discriminant`);
- **`==`, a `HashMap`'s key and `contains`** are JS's `===` (`js_eq`);
- **a derived order** is JS's `<`, by the numbers, as the derive orders
  by discriminants (`is_primitive_ord`); a hand-written one is its own;
- **`{:?}`** is its name, `["Empty", "Complex"][kind]` where they count
  from 0, else `{ "-1": "Low", 7: "Mid" }[level]`;
- **its `.d.ts`** is `0 | 1 | 2 | 3`; **an untagged enum** tells it as a
  number; **serde** writes and reads its name, as serde does, and makes
  the number of it (`de.rs`, by `variant_tag`);
- **a function giving each variant its name** is no longer its value
  (`gives_own_name`, ADR 0264), which it is only of a string enum.

A plain fieldless enum is its names, as before. So is one of a 64-bit
or 128-bit `repr`, whose integers are BigInts (ADR 0086), a tagged or
untagged one, and std's; and, until the second amendment below, one with
a variant named by `#[rust_js::name]`.

## Why

- **The `repr` says the number**: what the Rust writes is what the JS
  holds, as React's constants are, and what `as` gives is the value.
- **One place**: a variant's value was `variant_name`'s string in each
  consumer; each now asks `variant_tag`.

## Alternatives

- **Every fieldless enum its number**: what reads `"Ascending"` in a log
  reads `0`; ADR 0013's reasons hold where the program didn't ask for an
  integer.
- **Only with every discriminant written**: `#[repr(u8)]` already fixes
  the numbers rustc gives the rest.

## Consequences

- `enum_order`'s and `hash_values`'s snapshots read the numbers:
  `writeU8(state, small)`, and a constant comparison folded.

## Amended: each variant is its module's `const`

Case ([0262](0262-when-rust-and-js-disagree.md)): A: a variant read by
its `const`'s name is the same number as one written in place; only how
the JS reads changes.

### Context

React's `ReactLazy` keeps a lazy component's status as numbers, each a
constant of its module: `const Uninitialized = -1; const Pending = 0;
const Resolved = 1; const Rejected = 2;`, then `payload._status =
Pending` and `if (payload._status === Uninitialized)`. Its port's
`#[repr(i8)] enum Status { Uninitialized = -1, Pending = 0, .. }` was
the numbers in place, `payload._status = 0`, `if (payload._status ===
-1)`: the names the reader looks for were lost.

### Decision

**Each variant of a numbered enum is its module's `const` of its number,
named as the variant is, and each use of it in a body reads that name**,
as a `const` item is (ADR 0031): imported by its name where another
module reads it, and where another crate does (ADR 0770).

```rust
#[derive(Clone, Copy, PartialEq)]
#[repr(i8)]
enum Status {
    Uninitialized = -1,
    Pending = 0,
}

fn start(payload: &Payload) {
    if payload.status.get() == Status::Uninitialized {
        payload.status.set(Status::Pending);
    }
}
```

```js
const Uninitialized = -1;
const Pending = 0;

function start(payload) {
  if (payload._status === Uninitialized) {
    payload._status = Pending;
  }
}
```

- **The analysis names each one** an item of its enum's module
  (`analysis.rs`'s `variants`), named apart from the module's other
  names as any item is (`name_items`): two enums' `Pending` in one
  module are `Pending` and `Pending$1`, and one a JS global's,
  `Error`, is `Error$`.
- **One answer to how a body writes a variant without fields**:
  `FnCx::unit_variant`, its `const` by `fn_ref` where rust-js named one,
  the crate's or a library's, else its value (`bindings::unit_variant`).
  A test of it, a `switch`'s `case Pending:`, a value made, a `Default`,
  a deserialized one and a constant rustc computed in a body
  (`FnCx::const_value`) each ask it.
- **Declared where it's read or may be**: a `const` is written where a
  body reads it, or its enum is `pub` or another module or crate may
  read it, exported then, as a `const` item is; in its source's place,
  beside its enum. A `pub` enum's `.d.ts` declares each, `export const
  Pending: 0;`, beside its type.
- **An import is of what a module reads.** A test is written as it's
  lowered, recording its variant as read, then tidy may take it out: of
  an exhaustive `match`, the last arm's, `Dark` of `Light | Dark`, is
  what the others leave. An import of it would be of a name nothing
  reads, so the linker imports none of the crate's modules' items the
  module doesn't read, and the pipeline none of a library's.
- **Where the value is what's needed, it's the number**:
  `bindings::variant_tag` still says what each variant is. A module's
  `const` of one, `const DEFAULT: Status = Status::Pending;`, is `const
  DEFAULT = 0;`: a module's constants read no other, so nothing depends
  on which module loads first (ADR 0031). `{:?}`'s table is keyed by
  the numbers, its `.d.ts` type is `-1 | 0 | 1 | 2`, and serde writes
  its name. `as i32` is the value it casts, `status` of a variable; of
  a variant written, `Status::Pending as i32`, rustc's number, `0`,
  which it computed.
- **What a module's load makes reads another module's as its value**
  (case C, ADR 0262: a throw where Rust runs is a result changed):
  a thread-local's value, a static's or a constant's initializer lowered
  as code, and a `js::on_load!` run where their module loads
  (`CrateFacts::loads`). Where they read another of the crate's modules'
  variant or `const`, the two may import each other, and the one loaded
  first runs its load before the other has made its `const`s: `export
  const state = { value: Pending };` threw `Cannot access 'Pending' before
  initialization`. There each is its value, `{ value: 3 }`, as a module's
  constants are; of its own module's, made before it (`js::in_load_order`),
  or a library's, made before the crate loads, its name: `let phase =
  Early;`. `FnCx::reads_by_name` is the one answer, which `unit_variant`
  and `named_const` ask; it fixes the `const`s' too, ADR 0031's, whose
  thread-local `{ value: LIMIT }` threw so before this amendment.

### Why

- **Upstream's JS**: React names each status; a number in place loses
  the name a reader looks for, and reads as a magic number.
- **One way**: a variant is read by its name as a `const` is, through
  the same naming, imports and exports; no second way of naming a
  module's value.

### Alternatives

- **The numbers in place** (this ADR as first decided): what the program
  names, the JS doesn't.
- **A frozen object, `Status.Pending`**: TypeScript's `enum`'s shape, not
  React's, and an object read at each use.
- **A constant of a variant in a module's `const` by its name**: a
  module that reads another's at load may read it before it's made, in
  an import cycle; the number is the same value.

### Consequences

- Comparisons of two variants written, `Byte::Two < Byte::Max`, are no
  longer folded to `true`: `Two < Max`, as the Rust writes them.
- An `Option` of one compared to `Some(Kind::Empty)` is `kind == Empty`,
  where the number in place was `kind === 0`: `==` because the name
  isn't a literal the comparison knows is no `null`; the same answer.
- A `{:?}` of a variant written, `Counted::Second`, is its table's read
  of it, `["First", "Second"][Second]`, where the number's was folded.
- A function another module's load calls, in an import cycle, still
  reads its own module's variants and `const`s by their names, as a
  function reads a `const` (ADR 0031): if that module hasn't loaded yet,
  it throws. Load-time code calling into a cycle is the open case, a
  `const` item's as much as a variant's, for one decision of what a
  cycle's load may run (ADR 0592 refuses a dictionary's).
- A binding's import, a JS module's export, whose only read tidy took
  out isn't written either; a package nothing reads isn't loaded. Nor is
  a compiled std body's, `fused_iterator`'s `fuseDoubleEndedIterator_nextBack`,
  which no line of it read.

## Amended: a variant's `#[rust_js::name]` names its `const`

Case ([0262](0262-when-rust-and-js-disagree.md)): A: the variant is the
same number, read by another name; its `{:?}`, its order, its `as` and
serde are Rust's, by its Rust name.

### Context

React's `ReactCacheImpl` keeps a cache node's status as constants named
as JS names constants: `const UNTERMINATED = 0; const TERMINATED = 1;
const ERRORED = 2;`, then `s: UNTERMINATED` and `if (cacheNode.s ===
TERMINATED)`. Rust names a variant `Unterminated`, and the attribute that
says what an item is called in JS, `#[rust_js::name = "UNTERMINATED"]`,
made the enum a string enum: `s: 'UNTERMINATED'`, `case 'TERMINATED':`,
no `const`s, its `as u8` a table. The `#[repr(u8)]` that asked for the
number was dropped for the name, and the name a function, a field or a
`const` gets from the attribute was a value here.

### Decision

**A numbered enum's variant's `#[rust_js::name]` is the name of its
module's `const`**, as the attribute names a function or a `const` item:
the enum is still its numbers (`bindings::numbered` no longer asks the
variants' names), and the `const` is named, declared, exported and
imported by that name as any item is (`name_items`, by `fn_name`).

```rust
#[derive(Clone, Copy, PartialEq)]
#[repr(u8)]
enum Status {
    #[cfg_attr(rust_js, rust_js::name = "UNTERMINATED")]
    Unterminated = 0,
    #[cfg_attr(rust_js, rust_js::name = "TERMINATED")]
    Terminated = 1,
}
```

```js
const UNTERMINATED = 0;
const TERMINATED = 1;

function read(node) {
  if (node.s === TERMINATED) {
    return node.v;
  }
  ..
}
```

- **A name that isn't a JS name is refused**, as it can't name a
  `const`: "`Unterminated`'s `#[rust_js::name]` names its module's
  `const` of its number, .., so it's a JS name: `un-terminated` isn't
  one" (`bindings::validate`). One repeating the variant's own name is
  refused already (ADR 0573).
- **What reads the variant's Rust name still does**: `{:?}`'s table,
  `["Unterminated", "Terminated"][status]`, and serde's names.
- `#[rust_js::name = true]`, a discriminated union's (ADR 0284), is
  still not numbered.

### Why

- **Upstream's JS**: the reader finds React's names.
- **One meaning of the attribute**: what an item is called in JS. The
  value of a numbered variant is said by its `repr`; a string enum is
  written without one.

### Alternatives

- **Keep a named variant a string**: a `#[repr(u8)]` and a name could
  not both be had, and React's constants could not be written.
- **A name that isn't an identifier makes the enum its strings**: two
  meanings of one attribute, chosen by its spelling. Rejected; a string
  enum drops the `repr`.

## What holds it

- `test/lowering-structs-enums.test.ts`, "a repr enum's variant is its
  number" and "a repr enum's variants are its module's consts, each read
  by its name"; `test/cross-crate-items.test.ts`, "a library's repr
  enum's variant is imported by its name"; `test/corpus/repr_enum_numbers.rs`
  against native Rust; `test/corpus/load_cycle_values.rs`, two modules
  in an import cycle, against native Rust; the serde case "a repr enum serializes and
  deserializes by its variants' names".
- Mutations: `numbered-enum-named`, `numbered-enum-wide`,
  `numbered-discriminant-tested`, `numbered-debug-table`,
  `numbered-gives-own-name`, `numbered-ordered-by-name`,
  `numbered-declared-names`, `numbered-eq-structural`,
  `numbered-payload-untold`, `numbered-read-as-name`; and of the
  amendment `numbered-variants-unnamed`, `numbered-variant-const-undeclared`,
  `numbered-variant-const-unexported`, `numbered-variant-written-as-number`,
  `numbered-variant-tested-as-number`, `numbered-constant-as-number`,
  `numbered-variant-consts-undeclared`, `unread-module-imported`,
  `unread-library-item-imported`, `load-reads-by-name`,
  `load-reads-own-module-by-value`, `load-reads-const-by-name`,
  `load-reads-variant-by-name`; and of the second amendment, `numbered-given-name-a-string`
  and `numbered-given-name-unchecked`, which `test/lowering-structs-enums.test.ts`'s
  "a repr enum's variants are its module's consts, each read by its name"
  catches, and `test/corpus/repr_enum_given_names.rs` against native Rust.
