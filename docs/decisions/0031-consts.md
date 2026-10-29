# 0031. A `const` is the value rustc computed, under its own name

Status: Accepted. Amended: a constant whose value rustc can't say is its
initializer's, each use a copy where it may change (below); and so is
one written of other constants, as it's written (below). Amended by
[0770](0770-a-librarys-const-is-imported-by-its-name.md): a library's is
imported by its name. Extended by
[0722](0722-a-repr-enums-variant-is-its-number.md): a numbered enum's
variant is its module's `const` too, and what a module's load makes reads
another module's `const` as its value, as an import cycle may load it first.
Amended (2026-10-26): a `#[repr(transparent)]` struct's constructor of a
constant is its field, by its name. Amended (2026-10-26): an import cycle
through a JS file of the program's own is one (below). Amended
(2026-10-26): one written in a function's body is the function's (below).
Amended (2026-10-26): a `bool`'s read by its name is a boolean (below).

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
  A `const` inside a function went there too, next to the function
  (amended: it's the function's, below). Names follow the usual rules: a `const URL` in a module that uses the global
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
- **A `#[repr(transparent)]` struct's constructor of a constant is its
  field** (ADR 0584): `Lanes(SYNC_LANE)`, `SyncLane`, read by its name.

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
  (Amended below: and through a JS file of the program's own, one graph.)

Case N ([0262](0262-when-rust-and-js-disagree.md)): the value is the
same either way; read by its name, its module has made it.

Test: `lowering-constants`'s "a constant another module makes is read by
its name where its module loads first", run, a cycle's value too; the
corpus's `load_cycle_values`. Mutations `load-cycle-unseen`,
`load-relowered-twice` (`lower/pipeline.ts`), `load-reads-values`,
`load-reads-by-name` (`lower/items.ts`).

## Amendment: an import cycle through a JS file of the program's own

React's ReactFiberLane binds a JS file of React's own,
`#[link_name = "./src/ReactFiberDevToolsHook.js#isDevToolsPresent"]`.
That file imports ReactEventPriorities, a module of the crate's, which
imports ReactFiberLane and reads its lanes where it loads:

```js
export const NoEventPriority = NoLane;
```

The graph above had no edge through the JS file, so it saw no cycle, and
the port read `NoLane` by its name. An app that loads ReactFiberLane first
loads the hook, then ReactEventPriorities, before ReactFiberLane has made
`NoLane`: a `ReferenceError` (`undefined` under CommonJS), where Rust's
constant is always there.

**A module that imports a JS file of the program's own imports what that
file imports, and what those files do in turn.** It's read where the
crate's JS is, as the root's file names it (ADR 0028): beside the output,
and in source (ADR 0041). `output::LocalJs` reads each file once per build
with oxc (`to_oxc::loaded_modules`), and follows each relative specifier.
A module of the crate's is matched by its JS file's path, laid out as the
planner lays it out, with or without its extension, case-folded. Another
file is found as Node and bundlers find one, `./priority` as
`priority.js` or `priority/index.js`. React's port reads its values:

```js
export const NoEventPriority = 0;
```

- **What a file loads** is each `import`, `export .. from` and `export *
  from`, and each `import()` or `require()` of a string, wherever it's
  written, as a function it makes may run as it loads. Not `import type`,
  which loads nothing. It's read as JS with JSX, then as TypeScript, which
  reads most of Flow's types: React's hook is read so.
- **What can't be said may import any module of the crate's**: a file not
  found, unread, read as neither, one that loads `import(name)`, or a
  specifier that may name any file, `#internal` of a package's `imports`
  or `/src/a.js`. Reading values only takes edges away.
- **A package's imports none of the crate's**, `react`, `node:path`,
  `shared/ReactFeatureFlags`; nor does a style, data or an image,
  `./hook.css`. A package that imports the crate's own JS back, a cycle
  of packages, isn't seen: following one would mean resolving packages as
  each bundler does, for what's otherwise a cycle Cargo's dependencies
  don't allow between crates.
- **A module's `js::import!("./setup.js")`** (ADR 0110) is an edge too.
- **One graph**, `load_order::ModuleImports`, which `load_cycles` and
  `refuse_dictionaries_in_load_cycles` both ask: a dictionary in a cycle
  through a JS file is refused as one through the crate's own imports.
- **Lowering asks; artifact planning answers**, through
  `program::JsModules`: where each module's JS goes is the planner's, and
  only printing reads JS with oxc, so the front end uses no later phase.
  The driver hands lowering the planner's `LocalJs`, and the planner what
  it read.
- **The files read are what the build read**: of the manifest's sources,
  and of what Cargo is told the build read (`OutputPlan::bound`), so a
  change to a file's imports rebuilds the crate. The cache lives for one
  build; Cargo's record of the files is what's checked against them next
  time.
- **Not yet: a `pub use` re-export**, `export { one } from "./a.js"`, is
  no edge, and a cycle through one isn't seen (issue #295).

Case C ([0262](0262-when-rust-and-js-disagree.md)): the JS loaded in
another order would throw where Rust reads its constant, so Rust's value
is kept, paid for in output. It's a forced difference from React's own
JS, which reads `NoLane` by its name: React relies on what loads first.
`NoEventPriority = 0` holds whichever does.

Test: `lowering-constants`'s "a constant read where a module loads is its
value where a bound JS file imports back", run with the module loaded
first, through a file a directory down and extension-less, a file of its
own, a module, a package and a style of no cycle, a file not found, and a
`js::import!`; `lowering-dictionaries`'s "a dictionary that takes nothing
is its module's const", refused through a JS file; `to_oxc::loads`'s
`each_module_a_file_loads`. Mutations `bound-file-unfollowed`,
`bound-module-unmatched`, `package-may-import-any`,
`missing-import-ignored`, `missing-bound-file-imports-nothing`,
`style-may-import-any` (`output.ts`), `bound-file-imports-nothing`,
`unread-bound-file-imports-nothing` (`lower/load_order.ts`),
`effects-import-nothing` (`lower/pipeline.ts`), `bound-js-unread`
(`main.ts`), `export-from-loads-nothing`, `import-loads-nothing`
(`to_oxc/loads.ts`).

## Amendment: a constant written in a function's body is the function's

React's ReactFiberLane writes masks it needs once in the function that
reads them:

```js
export function includesBlockingLane(lanes: Lanes): boolean {
  const SyncDefaultLanes =
    SyncHydrationLane | SyncLane | InputContinuousHydrationLane | ..;
  return (lanes & SyncDefaultLanes) !== NoLanes;
}
```

Its port writes each a `const` in the function, as Rust does, and they
were the module's, beside it, the comment above one with it: the second
`SyncDefaultLanes`, `isBlockingLane`'s, was `SyncDefaultLanes$1`.

**A `const` written in a function's body is a `const` in it, where it's
written, in the block it's written in**, of a function's or a method's,
a closure's in the closure's, where nothing outside the function reads
it (`analysis/naming.rs::local_consts`): the function, its closures and
the functions written in it (ADR 0308) read it there, by its name.

- **Before what reads it first, where that's earlier**: Rust's `const` is
  read anywhere in its block, before it's written too; JS's only once
  it's made. What reads it is its name, or a function written there that
  reads it, which its block may call before it, so a `const` such a
  function reads is its block's first (`place_written`, the same placing
  as a function's, after the functions).
- **Its name is a name of its module's, as any item's, and preparation
  takes off a suffix nothing in its function needs**: `isBlockingLane`'s
  is `SyncDefaultLanes` again, and a variable of its function is named
  apart from it.
- **One something else reads is the module's, as before**: a method of a
  type written in the function, a static's initializer, another
  constant's initializer, which would read it before it's made, or a
  `const { .. }`'s. A thread-local's is its module's variable (ADR 0270),
  and one of a trait's default is its module's, as each impl copies its
  body into its own (ADR 0185). Its initializer runs where the function
  does, not where its module loads: it's none of `pipeline::load_cycles`'
  loads.
- **One read nowhere in JS, only by rustc, `[0; N]`'s, is dropped**, as a
  function's unread `const` is (ADR 0356): `nonzero.rs`'s `SEVEN` is no
  module `const` any more.
- **A hole is an item's**: `js::ExprKind::ItemHole`, which was a
  function's alone (ADR 0296), holds a `const`'s place too, filled by
  `pipeline/holes.rs`, which `pipeline.rs`, past its file's cap, moved
  into.

Case N ([0262](0262-when-rust-and-js-disagree.md)): the value is the
same, made where the function runs; the JS is upstream's.

Test: the corpus's `local_consts.rs`, beside native Rust, its JS each
function's `const`; `pointer_constants.rs`'s `LOCAL`, `type_id_statics.rs`'s
`F` and the `consts` snapshot's `STEP` are their functions' now.
Mutations `local-consts-none`, `local-const-read-outside`
(`lower/analysis/naming.ts`), `local-const-after-its-read`,
`local-const-after-a-call` (`lower/mir/closures.ts`),
`local-const-hole-unfilled` (`lower/pipeline/holes.ts`).

## Amendment: a dependency's JS isn't the crate's own source (#300)

The files `LocalJs` reads, `bound`, are what the build read, and Cargo's
dep-info lists them. Following imports, they took in a dependency's
published `lib.js` too. Deleting that file made Cargo rebuild the
crate, which then failed reading it with a bare "No such file or
directory", where the tooling refuses a deleted library's JS with
`cargo clean -p <package>`. `bound` now leaves out the dependencies' own
artifacts, which Cargo tracks through the dependency itself, and that
read's error names the file. `cargo-workspace`'s "a Cargo build of
rust-js's crates is done when their JS is" holds it again, and
`bound-of-dependencies-too` is caught.

## Amendment: a `bool`'s read by its name is a boolean

React's ReactFiberCacheComponent tests `enableCache`, a `bool` constant:
`ENABLED && COUNT.get() > 0.0` was `return !!ENABLED && count > 0;`, as a
read of a name, `ENABLED`, might be any JS value, which `&&` would give as
it is where it's falsy.

**A `bool` constant read by its name is known a boolean**, of the crate's
module, another's or a library's: lowering marks its read
`Known::Boolean`, which tidy's `is_boolean` reads as it reads a `bool`
variable's name, so `&&` and `||` give it as it is: `return ENABLED &&
count > 0;`, `return (LOGGED && !quiet) || TRACED;`.

Case N ([0262](0262-when-rust-and-js-disagree.md)): its value is `true`
or `false` alone, so `!!` of it is itself.

Tested by `test/lowering-constants.test.ts`'s "a bool constant read by its
name", run; its mutations are `bool-const-unknown` and
`known-boolean-unread`.

## Amendment: a re-export is an import (#295)

A `pub use` re-export, `pub use super::z::one`, is `export { one } from
"./z.js"`: an import of `z`. The graph of what each module imports, which
decides whether a module's load reads another's `const` by its name, had
no edge for one. So `z`'s `S = [K]` read `a`'s `K` by name though `a`
re-exports `z`, and where `a` loaded first, `z` ran before `a` made `K`:
`ReferenceError: Cannot access 'K' before initialization`.

`module_imports` now adds an edge from a module to the module of each item
it re-exports, read from rustc's module children. A re-export of a binding
adds one to the binding's JS module, `export { scale } from "./scale.js"`.

- `test/corpus/reexport_in_a_load_cycle.rs` checks a module's re-export
  against native Rust.
- lowering-constants' "a module re-exporting a binding imports its JS
  module, a cycle through it" checks the binding's.
- The mutations are `reexport-not-an-import` and
  `reexported-binding-not-an-import`.
