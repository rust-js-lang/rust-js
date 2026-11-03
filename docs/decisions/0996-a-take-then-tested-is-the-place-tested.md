# 0996. A `take()` then tested is the place tested, taken where it's `Some`

Status: Accepted. The user's decision (2026-10-28, React port gap tk1).
Amends [0586](0586-a-nullable-field-is-an-option.md), amended, whose items'
`take()` stores `null`: a `#[rust_js::nullable]` field's own `take()` does
too.

Case ([0262](0262-when-rust-and-js-disagree.md)): None. Of a place
rust-js made, nothing can observe the order: a second read of a plain data
property or a variable gives the same value, and `None` written over
`None` changes nothing, each place having one representation of it.

## Context

React's prepareFreshStack, restartGestureRoot and GestureScheduler take a
callback out of a root's field and call it:

```js
const cancelPendingCommit = root.cancelPendingCommit;
if (cancelPendingCommit !== null) {
  root.cancelPendingCommit = null;
  cancelPendingCommit();
}
```

Their port writes `if let Some(cancel) = root.cancel_pending_commit.take()
{ cancel(); }`, which printed the take first, `const c = x.f; x.f =
undefined; if (c != null) { c(); }`: it writes the field even where it was
`None`. And a `#[rust_js::nullable]` field's `take()`, or `mem::take` of
one, wrote `undefined`, where its `None` is `null`, as each other write of
it makes it (ADR 0772): JS reading `root.f === null` saw `false`.

## Decision

**A `take()` of a place rust-js made, then tested by an `if let` or a
`match`, is the place tested, and taken in its `Some` branch**:

```rust
if let Some(cancel) = root.cancel.take() {
    cancel();
}
```
```js
if (root.cancel != null) {
  const cancel = root.cancel;
  root.cancel = undefined;
  cancel();
}
```

- **Lowering says which takes may move** (`Known::Taken`), where types
  are known, not from the JS's shape: `Option::take` and `Cell::take` of
  an `Option`, of a variable of the body's, a field of the crate's struct
  or a `Cell`'s, a path of variables and fields none of whose reads a
  getter answers (`js::taken_place`). Not a `RefCell`'s value read
  through its borrow, nor an item, `v[i]`, nor a binding's property.
- **Tidy moves it** (`tidy/options.rs::taken_options`, the "taken
  options" pass): `const c = p; p = None; if (c != null) { a } else { b }`
  is `if (p != null) { const c = p; p = None; a } else { b }`, of `c == null`
  too, only where `c` is read nowhere but `a`.
- **A nullable field's `take()` stores `null`**, `root.pending.take()`
  and `mem::take(&mut root.pending)`, through each `&mut *r` MIR makes of
  another (`lends_nullable_place`).

## Why

- **It's upstream's JS**, which the port compares line by line, and the
  user's decision: a write only where there was something to take.
- **One owner each**: lowering decides a take may move, by the place's
  type; tidy only moves what it's told.

## Alternatives

- **Keep the take first**: correct, and not what upstream writes.
- **Move any take**: a binding's property may be a getter or a setter,
  which a second read or a skipped write would show.

## Consequences

The corpus's `mut_ref_escapes.js` and the scheduler port's
`workLoop` read `if (holder.held != null) { const x = holder.held; ..`.
An `if let` whose branches give a value, `if let Some(p) = r.p.take() {
p.len() } else { 0 }`, keeps its statements, `if .. else { return 0; }`.

## What holds it

`test/lowering-takes.test.ts`, "an Option taken then tested is the place
tested, taken where it's Some", run with each place set and not; the
mutations `option-take-unmarked` (`lower/mir/entries.ts`),
`cell-take-unmarked` (`lower/std_types/cell.ts`), `taken-binding-read-after`
(`lower/mir/tidy/options.ts`), `taken-options-off`
(`lower/mir/tidy/stages.ts`), `taken-place-any` (`js/stmt_walk.ts`),
`nullable-field-taken-undefined` and `nullable-reborrow-unfollowed`
(`lower/mir/rvalues.ts`).

## Amendment: a take written over before it's seen is no write (2026-11-02)

rd524, P3. React's Flight client takes a chunk's listeners and makes it
fulfilled, its state a `RefCell` its class's fields flatten (ADR 1044):

```rust
let listeners = match &mut *chunk.state.borrow_mut() {
    ChunkState::Pending { value } | ChunkState::Blocked { value } => value.take(),
    _ => None,
};
*chunk.state.borrow_mut() = ChunkState::Initialized { value: text };
```

```js
if (chunk.status === "pending" || chunk.status === "blocked") {
  const old = chunk.value;
  chunk.value = null;
  listeners = old;
} else {
  listeners = undefined;
}
chunk.status = "fulfilled";
chunk.value = text;
```

Upstream's `const listeners = chunk.value; chunk.status = 'fulfilled';
chunk.value = text;` writes no `null`: nothing reads it.

**A take's write of `None` over a place rust-js made (`Known::Taken`),
a field of a variable, that the program writes again before anything
could see it is no write** (`tidy/options.rs::overwritten_takes`, the
"overwritten takes" pass, after "handles in place", which makes a
flattened state's whole write its fields'). Between it and the write
over it, past the end of the `if` branch it's in too, each statement
writes a variable other than the object's, or another field of the
object, of a constant or another variable: nothing that reads the
object, the old value or anything else, runs code or throws.

```js
if (chunk.status === "pending" || chunk.status === "blocked") {
  const old = chunk.value;
  listeners = old;
} else {
  listeners = undefined;
}
chunk.status = "fulfilled";
chunk.value = text;
```

- **The fact is lowering's** (`Known::Taken`, above): the place is a
  plain data property or a variable, never a binding's, which a setter
  may answer, nor a cell's value read through its borrow: run 8 thought
  this needed a fact of MIR's, but the one this decision gives the take's
  write already says the place is rust-js's own.
- **What it doesn't drop**: a write the program makes twice, `s.n = 1;
  s.n = 2;`, which is no take's; a take past which a call runs, the
  object or the place is read, or the variable is made another's
  (`slot = other`); one only another field is written after. The last
  but one has no mutation: lowering marks no take of a variable the body
  assigns again, so the pass's own test of it is never what refuses, as
  the corpus's `moved_on` shows.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. Only a read of the
place could tell the `None` was there, and none runs before it's written
again.

Held by corpus `taken_then_overwritten` (the Flight client's take and
whole write; a struct's field taken and set again past another field;
one read between, in its branch too, given to a call, made another's,
only another field written after, and a field the program writes twice,
against native Rust), the changed `flattened_state_cell`,
`or_pattern_one_place` and `mut_param_moved_out`, and the mutations
`overwritten-any-write`, `overwritten-past-a-call`,
`overwritten-past-its-read`, `overwritten-same-field-between`,
`overwritten-branch-tail-unchecked`
and `overwritten-takes-off` (`lower/mir/tidy/options.ts`).

## Amendment: a take returned, whose reset nothing reads, is its value (2026-11-02)

rd598, P3. React's renderToString collects its chunks through its
destination's `push`, which the request keeps, and returns them:

```rust
let result = Rc::new(RefCell::new(String::new()));
let destination = Destination {
    push: {
        let result = Rc::clone(&result);
        Rc::new(move |chunk| {
            if let Some(chunk) = chunk {
                result.borrow_mut().push_str(chunk);
            }
            true
        })
    },
    ..
};
..
result.take()
```

The cell is a variable (ADR 0287), and its take was
`const old = result; result = ''; return old;`, where upstream's is
`return result;`.

**A take of a `let` the body declares at its top, and nowhere else,
returned, `const old = x; x = c; return old;` of a constant `c`, is
`return x;` where no closure, nor a function declared in the body,
reads `x`** (`tidy/values.rs::unread_resets`, the "unread resets" pass,
after "plain cells" and "overwritten takes"). Once the body returns,
only its closures can reach the variable, the `let` being the body's;
a closure that only writes it, `x = e`, or, of text or a number the take
left, appends to it, `x = x + e` or `x += e`, of an `e` that reads
nothing of it, sees nothing of what it holds. So no read follows the
reset, and a write nothing reads changes nothing. A `replace` of a
variable, `mem::take(&mut text)`, returned, is the same.

- **What keeps the reset**: a closure that reads the variable, kept past
  the take, `peek` of `move || result.borrow().clone()`, which sees what
  the take left; a `finally` that reads it, which runs after the
  `return`; a handle made on it (ADR 0099), which what it's given may
  read; an append where the take left `undefined`, which may be an
  object, whose `toString` the append calls; and a variable another
  closure or block declares again, or the body doesn't declare.
- **No lowering fact.** Unlike the overwritten take's, this is a fact of
  the JS alone: JS's scoping says what can reach a `let`, and each read of
  it is in the tree. A `Rc<Cell<u32>>`'s `count.set(count.get() + 1)`,
  `count = (count + 1) >>> 0`, reads it, so keeps its reset.
- The last three have no bun test that reaches them, and so no mutation:
  Rust's borrows keep a handle from outliving the body, a drop that reads
  a cell has it in a struct, which keeps it a box, and lowering makes no
  append of an `Option`. The unit test `unread_resets_returned`
  (`mir/tidy.rs`, `cargo test`) holds each.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. Only a read of the
variable after the `return` could tell the reset was there, and nothing
can make one. JS's limit on a string's length, which an append past the
take reaches from what was taken, not from `""`, is one rust-js models
nowhere.

Held by corpus `taken_and_returned` (React's destination kept past the
take, a closure called before it, a sum and an `Option` its kept
closures write, and a `mem::take`, each the variable returned; a reader
kept past the take, an appender beside a reader, a drop that reads the
cell, and a count its closure reads to bump, each against native Rust),
the unit test, and the mutations `unread-resets-off` and
`unread-resets-past-closure-reads` (`lower/mir/tidy/values.ts`).

## Amendment: a nullable place's by its handle (2026-11-04, #541)

That a `take()` writes a nullable field's or item's `None` as `null` is now
told by the `&mut` it's given. That's a handle that stores so
(`nulled::nullable_ref`), no longer `lends_nullable_place`'s walk of MIR's
reborrows ([ADR 1078](1078-a-nullable-place-stores-null-however-its-written.md)).
The JS is as it was.
