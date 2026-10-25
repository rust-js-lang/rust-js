# 0031. A `const` is the value rustc computed, under its own name

Status: Accepted. Amended: a constant whose value rustc can't say is its
initializer's, each use a copy where it may change (below); and so is
one written of other constants, as it's written (below). Amended by
[0770](0770-a-librarys-const-is-imported-by-its-name.md): a library's is
imported by its name. Extended by
[0722](0722-a-repr-enums-variant-is-its-number.md): a numbered enum's
variant is its module's `const` too, and what a module's load makes reads
another module's `const` as its value, as an import cycle may load it first.

Case: A, C, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

`const` items were an error. Programs want them for URLs, sizes, limits and
fixed tables, and so does the playground once it's written in Rust. In Rust,
a `const` is worked out at compile time, and each use is a value of its own,
as if the value were written there.

Two ways to get it into JS:

```text
const SIZE: u32 = 4 * 1024;
  (a) lower its expression   ─►  const SIZE = Math.imul(4, 1024) >>> 0;   computed as the module loads
  (b) ask rustc for the value ─►  const SIZE = 4096;                      computed at compile time
```

With (a), the order matters: a `const` that uses another declared later hits
JS's temporal dead zone. Across modules it's worse, since modules may import
each other (the `modules` example has `lib ↔ stats`), and one may run before
the other has set its constants. ReScript folds constant expressions too.
Scala.js inlines its `final val` constants.

## Decision

**rustc computes the value, and the JS declares it once, under the Rust
name.** Uses refer to it by name.

```rust
pub const SIZE: u32 = 4 * 1024;
const ORIGIN: Point = Point { x: 0, y: 0 };

pub fn size_in_kb() -> u32 { SIZE / 1024 }
pub fn moved(dx: i32) -> Point { let mut p = ORIGIN; p.x += dx; p }
```

```js
export const SIZE = 4096;
const ORIGIN = { x: 0, y: 0 };

export function size_in_kb() {
  return SIZE / 1024 >>> 0;
}
export function moved(dx) {
  let p = { ...ORIGIN };
  ...
}
```

- **The value comes from rustc's const evaluation**, as a value tree, and is
  written in the shapes the other ADRs give: numbers (0011), strings,
  `bool`, fieldless enum variants as strings (0013), structs and tuples as
  objects and arrays (0020), `undefined` for `None` (0030).
- **Each use is a value of its own**, so where a type is changed in place
  somewhere in the crate, a use copies (`{ ...ORIGIN }`), as reading a Copy
  place does (ADR 0020). Otherwise the name is used as it is.
- **Constants go at the top of their module's file**, before the functions,
  and are exported when they're `pub` or another module uses them (ADR 0019).
  A `const` inside a function goes there too, next to the function. Names
  follow the usual rules: a `const URL` in a module that uses the global
  `URL` becomes `URL$1`.
- **Other crates' constants are written in place**, as values: `u32::MAX` is
  `4294967295`.
- **A divisor rustc knows** needs no check for zero: `x / SIZE` is
  `x / SIZE >>> 0`, not `$div(x, SIZE)`.

## Why

- **It's what Rust means**: the value is fixed at compile time. A constant
  that doesn't compile (say, overflow) is already an error in rustc.
- **Nothing depends on load order**: literals don't read other modules, so
  cycles between modules and declaration order don't matter.
- **The JS keeps the names**: `x > SIZE` reads better than `x > 4096`.

## Alternatives

- **Lower the expression**, as in (a): the JS would show `4 * 1024`, but
  would have to order constants, and couldn't order them across module
  cycles.
- **Write every use as its value**, as Scala.js does: the simplest
  semantics, but the names are lost, and a big table would be repeated at
  every use.

## Consequences

- The JS shows the computed value, not how it was written.
- Only what a value tree holds can be a `const`: no `String` (a heap
  pointer), no function pointers, no `dyn`. Those are an error that says so.
- A type's own associated constants (`impl Foo { const N: u32 = 3; }`)
  are their value where they're used (ADR 0064).
- Not yet: `const` blocks. `static`s came later (ADR 0096), and a trait's
  associated constants (ADR 0106).
- **A constant borrowed, `&XS`, is the one there is**: nothing can change
  it through a shared reference, in this crate or another, so
  `XS.contains(&x)` is `XS.includes(x)`, not a copy of it first. One used
  by value, which may be changed, is copied still. (Amended: in a library,
  whose consumers might change an array, every use was a copy, `XS.slice()`,
  as react.dev's DownloadButton found.)

## Amendment: constants of one object's properties are its destructuring

Constants side by side, none exported nor set, that each read a property
of one variable are one destructuring of it, as react.dev's 404 page
writes `const {Intro, MaxWidth, p: P, a: A} = MDXComponents;`, which
was four `const`s. One alone stays `const P = T.p;`. A JSX test reads
two of one table and one of another; mutations keep them apart, and
destructure both tables' from the first.

## Amendment: a constant rustc can't say, used as a value of its own

A constant whose value rustc can't say as a value tree (2026-10-15),
`const EMPTY: Vec<i32> = Vec::new()`, is its initializer's, as a
static's is (ADR 0096), whatever its type: each use of a constant is a
value of its own, so its one JS value is copied at each use where it may
change, `EMPTY.slice()`. Which uses copy is `const_use_copied`'s, one
place: of a `Copy` type, where something changes one in place, as before;
of another, where it isn't shareable, a `Vec`'s, whose `push` changes it.
`const_copy` makes the copy, a clone of one that isn't `Copy`. A repeat
of one, `[EMPTY; 3]`, makes it again for each item,
`Array.from({ length: 3 }, () => EMPTY.slice())`; and `[x; 1]` of a value
that isn't `Copy` is the one there is, which Rust allows of any value.
The corpus's `const_block_vec.rs`, which was a compile-fail case of the
refusal, and rustc's `consts/const-blocks/const-repeat.rs` and
`array-slice-vec/issue-18425.rs`, pass.

## Amendment: a constant written of other constants is as it's written

React's ReactFiberLane and ReactFiberFlags write their masks of their
lanes and flags, and some of `shared`'s feature flags:

```js
export const SyncUpdateLanes: Lane = SyncLane | InputContinuousLane | DefaultLane;
export const SomeTransitionLane: Lane = TransitionLane1;
export const Hydrate = enableCreateEventHandleAPI ? .. : ..;
let nextTransitionUpdateLane: Lane = TransitionLane1;
```

Their port's constants were rustc's values, `SyncUpdateLanes = 42`,
`nextTransitionUpdateLane = 256`, `EventPriority(NO_LANE)`'s `[0]`: the
names were lost, and a flag's answer was baked in, where React's build
swaps a fork of `shared`'s flags in.

**A constant whose initializer names other constants read by their
names, the crate's or a library's (ADR 0770), and calls none of the
crate's functions, is its initializer, lowered as code, as a constant
rustc can't say is (above)**: `SyncLane | InputContinuousLane |
DefaultLane`, `[SyncLane]`, a `const { Cell::new(TransitionLane1) }`
thread-local's `TransitionLane1`, each read by its name.

- **Its order is ADR 0306's**: what it reads is made before it, in its
  module; a library's is made before the crate loads. Where a module's
  load reads another of the crate's modules' constant, it reads its value
  still (ADR 0722), as an import cycle may load it first.
- **A constant of literals is rustc's value, as before**: `4 * 1024` is
  `4096`, as a person writes it, where its expression would be
  `Math.imul(4, 1024) >>> 0`. Decided so (2026-10-26): the gap was names
  lost, and a literal keeps its text as it is (ADR 0627).
- **One that calls the crate's functions is rustc's value, as before**,
  `doubled(SIZE)`'s `8192`: what's worked out at compile time isn't run
  where the module loads. Decided so too (2026-10-26), the natural
  reading of a `const fn`'s call; a std function's call is no such run,
  `from_raw_parts(DANGLING, 0)`.
- **A static is too, where its initializer may be one** (ADR 0096): one
  of a type no one JS value can be every use's is refused, as before.
- **Where rustc needs the value, a pattern or an array's length**, it's
  rustc's still: `case SyncUpdateLanes:`, `[0, 0].length`.

Case N ([0262](0262-when-rust-and-js-disagree.md)): the value is the
same either way, rustc having worked it out, and a library's flag is
read from the library the crate was built with, unless a bundler swaps
it, as React's build means to.

Test: `lowering-constants`'s "a const written of other consts is as it's
written", run; `cross-crate-items`'s "a const written of a library's
consts imports them by name", run. Mutations `const-of-consts-folded`,
`const-of-library-consts-folded` (`lower/pipeline.ts`),
`const-calling-crate-code-as-code` (`lower/body_queries.ts`).

## Amendment: another module's constant is read by its name where its module loads first

React's ReactFiberHiddenContext makes its cursors where it loads, of
ReactFiberLane's lanes, which it imports:

```js
export const prevEntangledRenderLanesCursor = createCursor(NoLanes);
```

Its port read `createCursor(0)`: what a module runs where it loads read
another of the crate's modules' constant as its value (ADR 0722), as an
import cycle might load it before the other had made it, and the amendment
above left that "not yet".

**What a module runs where it loads reads another module's constant by
its name, where that module can't load after it: it doesn't reach this
one by what the crate's modules import of each other.** Of ES modules, one
another imports, of no cycle, has run before it. Where the two may import
each other, the value is read, as before.

- **Which is decided once the crate's bodies are lowered**, by what each
  imports of the others, `pipeline::load_cycles`: each body run where its
  module loads, reading by name another module's constant or numbered
  variant that reaches its module back, is lowered again reading their
  values (`CrateFacts::loads_in_cycles`, `FnCx::reads_by_name`). What it
  imported by name is an edge too, so the cycles found are each one that
  may be; reading values only takes edges away.
- **Of the imports `refuse_dictionaries_in_load_cycles` reads too**: each
  body's references of another module's items and its held bindings.

Case N ([0262](0262-when-rust-and-js-disagree.md)): the value is the
same either way; read by its name, its module has made it.

Test: `lowering-constants`'s "a constant another module makes is read by
its name where its module loads first", run, a cycle's value too; the
corpus's `load_cycle_values`. Mutations `load-cycle-unseen`,
`load-relowered-twice` (`lower/pipeline.ts`), `load-reads-values`,
`load-reads-by-name` (`lower/items.ts`).
