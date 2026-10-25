# 0270. A thread-local only read and set is its module's variable

Status: Accepted. Amends [0037](0037-thread-locals.md). Amended below: an
exported one is its `export let`, which only its module sets; one of an
array or an object its module sets whole is given the new one there.

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev's errors page caches the error codes it fetched in a variable of
its module:

```tsx
let cachedErrorCodes: Record<string, string> | null = null;
```

A Rust module keeps one in a `thread_local!`'s `Cell`, which rust-js made
a `{ value }` (ADR 0037), so anyone sharing the cell sees it change:

```js
const cachedErrorCodes = { value: undefined };
cachedErrorCodes.value = codes;
```

## Decision

**A thread-local only read and set, by `get`, `set`, `with_borrow` and
`with_borrow_mut`, in its own module, and not public, is its module's
variable of what its cell holds**: a `let` if it's `set`, a `const`
otherwise. (Amended by ADR 0328: a `with_borrow` whose closure may ask
whether the cell is borrowed keeps it its cell, borrowed while `f` runs.)

```rust
thread_local! {
    static COUNT: Cell<u32> = Cell::new(0);
    static LOG: RefCell<Vec<String>> = RefCell::new(Vec::new());
}
COUNT.set(COUNT.get() + 1);
LOG.with_borrow_mut(|log| log.push(line.to_string()));
```

```js
let COUNT = 0;
const LOG = [];
COUNT = (COUNT + 1) >>> 0;
```

- **One whose cell is handed out, `with(|cell| ..)`**, keeps its
  `{ value }`: what holds the cell must see it change.
- **One another module reads**, or that's public, keeps it: an importer
  can't set another module's `let`. (Amended below: it's its `export let`,
  and another module's write is refused.)

## Why

- **No program can tell**: nothing but its module's reads and sets reaches
  the cell, and each reads or sets the variable instead.
- **It's the JS a person writes.**
- **It's tested**: a compiler test sets a `Cell`'s and changes a
  `RefCell`'s `Vec`, beside a public one and one another module reads;
  mutations box them all, unbox the other module's, and make a set one a
  `const`. The thread-locals example and its snapshot say each.

## Since

- **A `const { .. }` thread-local is one too**, by its own name: std puts
  its value in a `const` of its block, `__RUST_STD_INTERNAL_INIT`, which was
  written by that name while the thread-local was read by its own, as
  react.dev's errors page's `cachedErrorCodes` found. And one that starts
  `None`, set later, is `let SEEN;`, as JS starts a variable `undefined`.

## Amendment: an exported one is its `export let`, which only its module sets

Case: C ([0262](0262-when-rust-and-js-disagree.md)) for what a JS
importer reads, which a `{ value }` changed; A for the rest: each module
reads and sets what it did. The user's choice (2026-10-25) of the forks
the React port's BadMapPolyfill found: refuse another module's write, not
a setter rust-js writes, nor a box kept where one writes.

React's BadMapPolyfill exports `let hasBadMapPolyfill`, which it sets as
it loads, for an importer to test, `!hasBadMapPolyfill`. Its port's `pub
static HAS_BAD_MAP_POLYFILL: Cell<Option<bool>>` was `export const
hasBadMapPolyfill = {}`, whose `value` it set: a JS importer's
`!hasBadMapPolyfill` was always `false`, and its `.d.ts` said `{ value:
boolean | null | undefined }`, not upstream's `boolean`. React writes 70
such `export let`s, each set only in its module, as JS lets only that
module set it, and exports a function that sets it where another must,
`setIsRendering`.

**A thread-local of a `Cell`, or of a `RefCell` of what isn't the object
it holds (ADR 0863), is its module's variable, exported or not, wherever
its module only reads and sets it and every other module only reads it:**

```rust
thread_local! {
    pub static HAS_BAD_MAP_POLYFILL: Cell<Option<bool>> = const { Cell::new(None) };
}
js::on_load! {
    if __DEV__ {
        HAS_BAD_MAP_POLYFILL.set(Some(false));
        ..
```
```js
export let hasBadMapPolyfill;

if (__DEV__) {
  hasBadMapPolyfill = false;
```

- **Its module's own rule is this ADR's**: one its module hands out,
  `with(|cell| ..)`, keeps its `{ value }`; one nothing sets is `export
  const limit = 4;`. Its `.d.ts` is what its cell holds, `export let
  enabled: boolean;`.
- **Another module reads it by its name**, `import { enabled } from
  "./flags.js"`, ES's live binding: `get` is `enabled`, a quiet
  `with_borrow(f)` is `f`'s body of `label` (`imported_variables`). Such a
  read is `Known::Live`: what it is where it's read, never a `const` of it
  made earlier read in its place, as `prepare/reclaim.rs` made `const
  rendering = isRendering` the import by that name. So a `let current =
  fiber::CURRENT.get();` that's read after its module may set it is
  `const current = current$1;`, its import named apart: the readability
  share of renamed names grows by `exported_cells.rs`'s, blessed for it.
- **Another module's write is refused**, in the crate or a consumer's: a
  `set`, `replace` or `take`, a `with_borrow_mut`, or its cell handed out,
  `with` or `&KEY` (`analysis/plain_locals.rs::outside_writes`). The error
  says to write a `pub fn` in its module that sets it, as React's
  ReactCurrentFiber does. That holds of one that's still a `{ value }`
  too, so what another module may do doesn't hang on how the cell's own
  module uses it. A `RefCell` of an array or an object is that object,
  which any module changes in place, as JS's importer of it may:
  React's taint registry's `Set`s.
- **A library's manifest says which are variables**, `Calling.variable`:
  its consumers read one by its name and are refused a write, and a
  consumer's `with_borrow` of one only where what it runs is quiet, as
  nothing counts its borrows.

Alternatives, which the user weighed:

- **A setter rust-js writes**, `export function setHasBadMapPolyfill(v)`,
  that another module calls: an export upstream has none of, where a
  library can't know which a consumer will set, so it would write one of
  every exported cell it sets.
- **Its `{ value }` kept where another module writes it**: two forms of
  one construct, which JS's importer can't tell apart, chosen by what
  some other module does.

`static mut` (ADR 0096) is `{ value }` in its own module too, written
from any: the same rule would make each its module's `let`, and refuse
another module's write, which `static_mut.rs` makes. Left as it is, for
the user to decide.

What holds it: `test/lowering-module-variables.test.ts`, a crate's modules, a JS
module importing one, each refused write, and a library and its
consumers, React's BadMapPolyfill's JS; `test/corpus/exported_cells.rs`,
against native Rust, a read kept before its module sets it again. The
mutations `outside-*`, `exported-cell-boxed`, `importer-*`,
`live-import-aliased`, `*-variable-read-as-cell`,
`contract-variable-unsaid` and `variable-declared-*`.

## Amendment: one of an array or an object its module sets whole is given the new one

Case ([0262](0262-when-rust-and-js-disagree.md)): A. React keeps
`let callbacks = []` and empties it, `callbacks = []`. A thread-local
`RefCell<Vec<_>>` that's its module's variable is the array (ADR 0863);
its `set` wrote it in place, `$assign($borrowMut(callbacks), [])`, its
borrow checked where its type's were elsewhere.

**In its own module, a whole write of one is the variable given the new
one**, `callbacks = []`, `export let callbacks`: its borrows end where
they're made (it's quiet, `quiet_locals`), so nothing else holds the old
one, and no run tells the two apart. Its `set` there borrows nothing
(MIR's `QuietCells`, ADR 0863, amended). Another module, which can't set
its import, writes the one it reads in place, `$assign(callbacks, [v])`,
which the live binding shares, as before.

`test/lowering-module-variables.test.ts`, "a thread-local of an array its
module sets is its export let", read from a JS module; corpus
`thread_local_refcell_set`'s `ports = [1, 0]`. Mutations
`module-array-set-in-place`, `module-array-set-const` and
`module-array-set-unexported` (`std_types/cell.rs`).
