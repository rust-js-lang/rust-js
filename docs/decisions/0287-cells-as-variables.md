# 0287. A cell only its function reads and sets is a `let`

Status: Accepted. Extends [0023](0023-strings-references-shared-state.md)
and [0270](0270-module-variables.md). Amended by
[0791](0791-a-cells-storage-says-what-it-stores.md): a handle on a field's
`Cell` written through, a closure's `r.set(5)`, is its place where it's
used, never read where it's made (#192).

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A `Cell` is `{ value }` (ADR 0023), so everyone sharing it sees a change.
A local one usually shares nothing JS doesn't share already: JS closures
capture variables, not values. react.dev's Preview keeps `let timeout`,
which its bundler listener sets and its cleanup clears. Rust needs an
`Rc<Cell<_>>` for that, and a clone of it for each closure, which was:

```js
const timeout = {};
const started = timeout;
listen((message) => { started.value = setTimeout(..); });
return () => { clearTimeout(timeout.value); };
```

ADR 0270 made a thread-local only read and set its module's variable.

## Decision

**A local `Cell`, or an `Rc` of one, that only its function and its
closures read and set, by `get` and `set`, is its function's `let`, and a
clone of it, `let started = timeout.clone()`, is that variable too.**

```js
let timeout;
listen((message) => { timeout = setTimeout(..); });
return () => { clearTimeout(timeout); };
```

- `let n = Cell::new(x)` and `let n = Rc::new(Cell::new(x))` are `let n
  = x`; `n.get()` is `n`, and `n.set(v)` is `n = v`.
- A closure captures it, `move` or not, as JS does: it isn't copied.
- One used any other way is a cell, and so is each of its clones: given
  or returned whole, `replace`d, `take`n, compared, or cloned anywhere
  but a `let` of its own.
- It changes, so what reads it is read where Rust reads it: JSX takes
  `{n.get()}` before a later `n.set(5)` (ADR 0218).
- `let x = undefined;` is `let x;`, as a person writes it.

## Why

- **It's the JS a person writes**, and the same program: a variable
  every closure captures is one place, as the cell was.
- **Conservative**: anything but `get`, `set` and a clone is a cell, as
  before.

## Consequences

- The corpus, examples and the playground lose their `{ value }`s of
  counters and flags: `let calls = 0; calls = (calls + 1) | 0;`.

## Amendment: a closure's cells are its own body's (2026-11-02)

`plain_cells` finds the cells and clones of the body it's given in that
body's own statement lists, not in its closures'. A closure's body is
tidied on its own before its function's, so a closure's cell is already
its `let` by the time the function is tidied. The nightly of 2026-11-02
(run 41360040940) found the mutation `mir-tidy-plain-cells-not-in-closures`
surviving: the function's walk into its closures changed nothing. A probe
that logged each closure's clone it found, of a cell, that the body's own
lists didn't have saw none: not in `check:fast --all`, and not in the
React port's 13 crates. The mutation is gone. lowering-cells' "a cell
cloned in a closure is its function's let" still holds `let running =
false;` and `let total = 0;`.

## Amendment: a `let` of a shared borrow keeps its name (rd658)

React's ReplyBackingFormData reads a `RefCell` field through a `let` of
its guard, `let string_chunks = backing_store.string_chunks.borrow();`,
then a call, `string_chunks.get(key)`. A field's `RefCell` no borrow of
which meets another is a handle on the field (ADR 0863, amended), and the
handle was read where it's made only where nothing that could set a field
ran before its last use (`read_at_once`): past the call, each use was the
field again, `backingStore.stringChunks.get(key)`, and the `let`'s name
was lost, where upstream writes `const stringChunks =
backingStore.stringChunks;`. getBackingBlob kept its `blobs` by chance:
its MIR held `blobs.as_ref()`, of a map of JS objects, in a temporary read
right after the `let`, `const _5 = blobs.value;`, the handle's only use,
before any call, which the inner `Some(blobs)` then named.

**A `let` of a shared borrow, a `&T` or a `RefCell`'s `Ref<T>`, of a
`T` with no cell of its own (`Freeze`), is read where it's made and keeps
its name** (`LetNames::frozen`, which `read_handles` reads), whatever
runs before its uses: while it's held, Rust lets nothing change what it's
to, a `RefCell`'s `borrow_mut` panicking where rust-js counts borrows and
proven unable to meet it where it doesn't. A `&Cell`'s, `let count =
&store.count;`, whose `set` a call it's held across may run, still reads
its place where it's used, and so does any other handle MIR makes. Case
([0262](0262-when-rust-and-js-disagree.md)): none: the value read where
the `let` is is the one each use would read.

Held by the corpus's `frozen_borrow_named.rs`, against native Rust (a
`Ref` held across calls that set the store's other cells and a variable
it was borrowed through given another store; a `&Cell` across a call that
sets it), and the mutations `mir-tidy-frozen-handle-at-its-uses`
(`mir/tidy/values.ts`), `frozen-borrow-of-a-cell` and
`frozen-borrow-not-of-a-ref` (`mir/patterns.ts`).
