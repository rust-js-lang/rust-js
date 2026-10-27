# 0270. A thread-local only read and set is its module's variable

Status: Accepted. Amends [0037](0037-thread-locals.md). Amended below: an
exported one is its `export let`, which only its module sets; one of an
array or an object its module sets whole is given the new one there; a
`with_borrow_mut` given its `{ value }`, and a `with` its cell, used in
place, leave it its variable, and an exported one its module needs the
cell of is refused; and a `static mut` is its module's variable by the same
rule (ADR 0096, amended); and one set in a closure a `const` is made of,
an impl's too, is set there.

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

`static mut` (ADR 0096) was `{ value }` in its own module too, written
from any. The user decided (2026-10-26) it follows this rule: its module's
`let`, `export let` where it's `pub`, read by its importers by name, and
another module's write refused, `holds_a_value`'s and `outside_writes`'
as a thread-local's is; `static_mut.rs` sets it by a `pub fn` of its
module now (ADR 0096, amended). Where its module gives on a pointer to
it, a handle on its box, one it exports is refused as one here its module
needs the cell of is (below).

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

## Amendment: what a borrow lends, used in place, is the variable; an exported one that needs its cell is refused

Case ([0262](0262-when-rust-and-js-disagree.md)): C for what a JS
importer reads, which the box changed, a wrong answer; A for the rest,
each module reading and setting what it did.

React's ReactFiberTransition imports `entangledTransitionTypes`, an
`export let` of ReactFiberTransitionTypes that starts `null`, and tests
it `!== null` before it steps it. Its port fills the `Option` where it's
first needed:

```rust
thread_local! {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub static ENTANGLED: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}

pub fn entangle(types: &[String]) {
    ENTANGLED.with_borrow_mut(|entangled| {
        let queued = entangled.get_or_insert_with(Vec::new);
        ..
```

A `with_borrow_mut` of a number, text or an `Option` gives its closure
the cell's `{ value }`, the box its `&mut` is (ADR 0074), which
`plain_statics` left a `$withBorrowMut(entangled, (entangled) => {
entangled.value ??= []; .. }, true)`: the cell stayed its box, `export
const entangled = { value: null }`, which the importer's `!== null`
always passed, and its `for (const t of ..)` then threw. React's
`ReactCurrentFiber`'s `current`, set so, was `export const current = {}`
too (`refcell_field_places`).

**A quiet `with_borrow_mut`'s closure given the box is called with the
cell itself, in place; where its body reads and sets the box only by its
`value`, the box is the variable**, as one only set is:

```js
export let entangled = null;

export function entangle(types) {
  entangled ??= [];
  const queued = entangled;
  ..
```

So are `Rc::clone(shared.get_or_insert_with(..))`, `get_or_insert_default()`
and `*n += 1`. **A `with` is read as a `with_borrow` is**: what it runs
must be quiet of its key, and lent the cell itself, used in place,
`COUNT.with(|count| count.set(count.get() + 1))`, is `count = (count + 1)
>>> 0`.

**Where its module needs the cell itself, one it exports is refused**, at
its declaration: it hands on the cell, `with(bump)`, or the `&mut` of what
it holds, `with_borrow_mut(|depth| fill(depth))` or `with_borrow_mut(fill)`;
or it borrows it in a `with`, `with(|label| label.borrow().len())`, or
again while a `with_borrow` of it runs (ADR 0328). The error says to read
and set it with `get`, `set`, `with_borrow` and `with_borrow_mut`, using
in their closures what they give, or to keep it in its module, read by a
`pub fn`. This takes the place of "one its module hands out keeps its
`{ value }`" above for one it exports; one it keeps is its box, as
before, as nothing outside reads it.

- **A counted `Rc`** (ADR 0852) that `get_or_insert_with` writes over may
  run a destructor that `cell_borrows` can't see through, so its closure
  isn't quiet, and an exported one is refused.

Alternatives:

- **Exporting the box**, as before: a JS importer reads an object where
  Rust has the value, a wrong answer, which AGENTS.md ranks first.
- **A copy of the variable in a box of its own where it's handed on**,
  `const d = { value: depth }; fill(d); depth = d.value;`: a box and a copy
  back upstream doesn't write, at each call, for a form the port can write
  in the closure; left for a port that needs it.
- **A setter written for it**: rejected above, for its export upstream
  has none of.

The corpus reads shorter: `refcell_field_places`' `current` and
`thread_local_refcell_set`'s `count` are variables, their
`$withBorrowMut`s gone.

What holds it: `test/lowering-module-variables.test.ts`, "an exported
Option a with_borrow_mut fills in place is its export let", read from a
JS module, and "an exported cell its module needs the cell of is
refused", each form refused, kept where it's private, and a `with` used in
place a variable; `test/modules.test.ts`, "MIR's thread-local number
changed by with_borrow_mut is its let"; corpus
`exported_cell_filled_in_place`, against native Rust. Mutations
`mir-quiet-box-kept`, `kept-box-refused` (`std_types/cell.rs`),
`exported-box-unrefused` (`pipeline.rs`), `local-with-never-quiet` and
`local-with-unasked` (`analysis/plain_locals.rs`).

## Since: a `static mut` is one too

A `static mut` is its module's state as a thread-local of a value is, one
rule for both (ADR 0096, amended): `holds_a_value` says so of one of the
crate's or a library's, `plain_statics` makes it its module's variable,
`outside_writes` refuses another module's write of it, whole, of a field
or an item, or a raw pointer taken to it, and one its module exports that
a pointer it gives on keeps in its box is refused at its declaration
(`refuse_boxed_export`). A library's manifest says it's a variable,
`Calling.variable`, as of a thread-local.

## Amendment (2026-10-27): set in a closure a `const` is made of

A thread-local set only in a closure a `const` is made of, a closure
coerced to a `fn`, `const BUMP: fn() = || TRIES.set(TRIES.get() + 1)`,
or a struct's `fn` field, React's DEV dispatchers' `currentHookNameInDev`
in each hook's, was `const tries = 0;`, which the closure reassigned:
`tries = (tries + 1) >>> 0`, a `TypeError` where Rust counts. What sets
it was looked for in functions and in what the module runs when it's
loaded, not in a `const`'s value. And in an impl's `const`, a namespace's
constant, `tries.value` wasn't even made `tries`: a property set on a
number, a `TypeError` in a module too. Both a wrong answer.

**What sets a module's variable, and what reads its `value`, is looked
for in every closure, a `const`'s and an impl's `const`'s too**: the
item walkers, `js::each_item_expr_mut` and `js::each_item_block_mut`,
walk a namespace's constants as they walk a module's, and `plain_statics`
counts sets, and `$assign`s made the variable's `=`, with
`each_item_block_mut`. Case A: Rust's behavior, preserved.

```js
let tries = 0;

const Counter = {
  BUMP: () => {
    tries = (tries + 1) >>> 0;
  },
};
```

Held by `test/corpus/thread_local_set_in_const_closure.rs`, against
native Rust; the mutations `item-blocks-skip-constants`,
`item-exprs-skip-namespace-constants` and
`item-blocks-skip-namespace-constants` (`js/stmt_walk.ts`).
