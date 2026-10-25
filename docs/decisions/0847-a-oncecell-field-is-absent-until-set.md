# 0847. A `OnceCell` in a field is the property, absent until it's set

Status: Accepted. Amends [0317](0317-once-cells.md), whose `OnceCell` was a
`{ value }` everywhere and set by `$onceSet`; extends
[0288](0288-cell-fields.md)'s `Cell` field to a `OnceCell`'s, and
[0576](0576-what-a-bodys-integers-can-be.md)'s walk of a body to which `OnceCell`s are
still empty.

Case ([0262](0262-when-rust-and-js-disagree.md)): A. The program reads
the cell as it did, `None` until it's set and `Err` of a second `set`;
only how JS holds and sets it changes.

## Context

React's `lazy` makes its payload without its DEV-only field and adds it
in DEV:

```js
const payload = { _status: Uninitialized, _result: ctor };
..
if (__DEV__) {
  const ioInfo = { name: 'lazy', start: -1, end: -1, .. };
  payload._ioInfo = ioInfo;
}
..
const ioInfo = payload._ioInfo;
if (ioInfo != null) { .. }
```

The port's idiomatic field for "set once, later" is a `OnceCell`,
`io_info: OnceCell<Rc<ReactIOInfo>>`, made `OnceCell::new()`, set by `let _
= payload.io_info.set(Rc::clone(&io_info))` and read by `if let Some(io_info)
= payload.io_info.get()`. ADR 0317 boxed it, as a `OnceCell` of its own:

```js
const payload = { _status: Uninitialized, _result: ctor, _ioInfo: {} };
$onceSet(payload._ioInfo, ioInfo);
const ioInfo = payload._ioInfo.value;
```

A box, a helper and a property that's there in production, where
upstream's object hasn't one, are a port's gap.

The user's decision: a `OnceCell` field is the plain property, absent
until it's set, as an optional field is; `set` is an assignment where
rust-js proves the cell still empty, and elsewhere Rust's own check and
`Result`; a `T` whose JS may be `undefined` keeps a correct form.

## Decision

**A `OnceCell` or a `OnceLock` in a struct's or a variant's field is the
property its `Option` is, as a `Cell`'s field is the value it holds (ADR
0288): left out where it's made `OnceCell::new()`, `undefined` until it's
set.** `get()` and `into_inner()` read the property, `None` where it's
`undefined`; `take()` reads it and makes it `undefined` again, as a
`Cell`'s `take` does; `get_or_init(f)` and `get_mut()` have a handle on
it, as every helper given a `Cell` field has (ADR 0099). A `OnceCell` of
its own is still `{ value }`, and one only its function reads and sets is
its `let` (ADR 0287).

**`cell.set(v)` is an assignment, and `Ok`, where the body's walk shows
the cell still empty; elsewhere it's `if (cell === undefined) { cell = v;
}`, and its `Result`, where the program reads it, `Err(v)` where it was
set:**

```js
payload._ioInfo = ioInfo;              // known empty

if (node.cache === undefined) {        // its Result unread
  node.cache = value;
}

const unset = node.cache === undefined;  // its Result read
if (unset) {
  node.cache = value;
}
const set = unset ? { TAG: "Ok" } : { TAG: "Err", _0: value };
```

- **Still empty** is what the integer ranges' walk of the body (ADR 0576)
  now knows too: a cell is empty from where a `let` makes it,
  `OnceCell::new()` in a struct literal, through `Rc::new` or `Box::new`,
  `let payload = Rc::new(Payload { .., io_info: OnceCell::new() })`, until
  anything could set it. Only what has the cell could: the variable that
  made it, and each `let` made of that one as it is or of a clone of its
  `Rc`, in a struct, a tuple or an array, `let lazy = Lazy { payload:
  Rc::clone(&payload), .. }`. Any other read of one of those, given to a
  call, lent, borrowed raw, captured by a closure, set or matched, and a
  `set` through any but the variable that made it, forgets the cells it
  reaches; so does a `let` of them whose drop may run code of the
  crate's, a binding's, a `dyn`'s or a type parameter's, which a `Drop`
  guard's would. A `get()` or `set()` of the cell itself gives nothing
  away. Paths join as the walk's do, so a cell set on one branch, or on
  an earlier turn of a loop, is no longer known empty; a `set` is
  assigned only where it's empty every time it's reached. React's
  `lazy`'s DEV `set` is, past `console.createTask` and `new Error(..)`,
  which can't reach a payload nothing gave them.
- **What it's set to is made first**, as Rust makes it before the call:
  held in a `const` where it does something, `const value = made("two");`,
  and where the `Result` is read, which gives it back as `Err`.
- **Of a `T` that looks like `None`**, `()` or an `Option`, what's set is
  `$some(v)`, boxed (ADR 0051), so a cell set is never `undefined`, and
  absence still means empty: `flags.unit = $some(undefined)`.
- **Declared**, the field is optional, `ioInfo?: ReactIOInfo | undefined`,
  as Flow's `_ioInfo?: ReactIOInfo` is.
- **`$onceSet` is gone**: every `set` is the assignment or the test.
- A `#[derive(Clone)]`'s spread copies the property, and a derived `==`,
  `Debug` and `Default` read and make it as the `Option` it holds.

## Why

- **Upstream's object**: no `_ioInfo` in production, `payload._ioInfo =
  ioInfo` in DEV, and `payload._ioInfo` where it's read, each as React
  writes it.
- **It's the same program**: a `set` that may find the cell set still
  tests it, and its `Err` gives the value back; one assigned is one no
  code could have reached since the cell was made empty.
- **One walk, one owner**: the body's walk already follows each path,
  each loop to its fixpoint and each `break` to its target, for integers;
  what's empty is one more fact of what's known where the body is, not a
  second walk of its own.

## Alternatives

- **Always test**, `if (payload._ioInfo === undefined)`: correct, but not
  upstream's, and the user asked for the assignment where it's proven.
- **Escape analysis by type**, ADR 0580's summaries of what each body
  may borrow: a binding's call, `console.createTask`, may run anything,
  so React's `set` couldn't be proven; that the payload is fresh, and
  given to nothing, is what proves it.
- **A `OnceCell` of a `T` that looks like `None` refused**, or kept `{
  value }`: the box `$some` makes keeps absence as empty already.

## Consequences

- A local `OnceCell` set where it's known empty, `let local =
  OnceCell::new(); local.set(1)`, is a `let` set once, `let local = 1;`.
- A `set` whose `Result` is read holds its test, `const unset = ..`, so a
  function of several reads `unset$1`, `unset$2`: `docs/readability.txt`'s
  `renamed x$1` share grew from 745.5 to 748.8 in 10,000 lines, blessed
  here, mostly of the two corpus cases that set one cell after another.
- `get_or_init` of a field gives `$getOrInit` a handle on it, `{ get
  value() { .. }, set value(v) { .. } }`: correct, but not what a person
  writes; its reentrant init (ADR 0317) is what keeps it a helper.

## What holds it

- `test/corpus/once_cell_fields.rs`, against native Rust: React's `lazy`
  made with and without its DEV field, read, set again (`Err`), a struct
  of `OnceCell`s of a number, `()`, an `Option` and a `String` made by
  `Default`, cloned, compared, shown, set, taken, `get_or_init`, a field's
  `get_mut`, taken apart in a parameter, and `into_inner`.
- `test/corpus/once_cell_known_empty.rs`, against native Rust: a cell
  made and set at once, and set again; given to a call, through a clone a
  struct holds, a clone kept unused, a holder dropped that runs nothing,
  one set through a holder, one through a raw pointer, a closure that sets
  it, a `Drop` guard dropped at a block's end and at a loop's `break`, a
  set on one branch, sets in a loop of a cell made each turn and of one
  made before it, values made with effects whether set or not, a local
  set twice, and a cell a call made.
- `test/declarations.test.ts`, "declarations type what a module exports":
  `ioInfo?: string | undefined`.
- Mutations: `once-set-unchecked`, `once-set-unboxed`,
  `once-set-value-made-when-unset`, `once-set-result-tested-held`,
  `once-take-keeps` (`std_types/once.rs`); `once-set-leaves-empty`,
  `once-set-through-holder-unforgotten`, `once-set-judged-once`,
  `once-mention-quiet`, `once-place-unmentioned`,
  `once-holder-reaches-nothing`, `once-raw-borrow-unread`,
  `once-made-unknown` (`body_queries/ranges.rs`);
  `once-drop-runs-nothing`, `once-made-by-any-call`,
  `once-rc-clone-gives-cells`, `once-rc-deref-unfollowed`
  (`body_queries/ranges/empty.rs`); `once-field-boxed`
  (`representation.rs`); `once-field-declared-a-cell`
  (`declarations.rs`); `once-set-never-known-empty`,
  `once-set-result-always-read` (`mir/std_calls.rs`);
  `clone-cell-field-respread` (`std_impls.rs`).
