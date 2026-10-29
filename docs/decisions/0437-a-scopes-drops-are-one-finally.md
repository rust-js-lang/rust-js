# 0437. A scope's drops on a panic are one `try { .. } finally { .. }`

Status: Accepted. Extends [0098](0098-destructors.md) and
[0372](0372-a-check-that-drops.md). Amended by
[0693](0693-a-guards-scope-is-one-finally-its-returns-in-it.md): a run
holds `if`s, loops and labeled blocks, and leaves from inside them where
each way out drops the value first.

Case: A ([0262](0262-when-rust-and-js-disagree.md)): the same drops run, in
the same order, on every path; only how the JS reads changes.

## Context

A panic drops what's live as it unwinds (ADR 0098). MIR gives each call
that may panic its own cleanup, so rust-js wrapped each statement that may
throw while a value with a destructor is live in its own `try`, whose
`catch` drops it and throws on, and the scope's end dropped it again:

```js
const shapes = [..];
try {
  console.log(total(shapes));
} catch (error) {
  for (const item of shapes) item.impl.$drop?.(item.value);
  throw error;
}
try {
  console.log("end");
} catch (error) {
  for (const item of shapes) item.impl.$drop?.(item.value);
  throw error;
}
for (const item of shapes) item.impl.$drop?.(item.value);
```

Each drop was written three times over, and the corpus's programs with
destructors were mostly this.

## Decision

**A run of statements a value's drop ends, each of which that may throw
drops it last in its `catch`, is one `try { .. } finally { drop }`**, as a
person writes cleanup:

```js
const shapes = [..];
try {
  console.log(total(shapes));
  console.log("end");
} finally {
  for (const item of shapes) item.impl.$drop?.(item.value);
}
```

- **Inside out**: a value made inside the run is its own scope inside it,
  `try { try { .. } finally { drop(e) } } finally { drop(d) }`, so a panic
  drops the newest first, as Rust's unwinding does. A `catch` left with
  nothing but its `throw` is no `try`.
- **The same drops**: a statement's `catch` drops the scope's value last,
  and the scope's end drops it, the two written alike up to where each was
  written and what each names what it declares (`js::stmts_alike`), so
  `for (const item$1 of v)` is `for (const item of v)`. It runs once the
  temporaries are named (`tidy::named`).
- **Only a scope**: a run stops at a statement that may throw outside any
  `try`, and at one whose `catch` drops another value last, one whose life
  crosses this one's, `drop(first)` while `second` is live, which keeps its
  `try` per statement. What the run declares and what follows reads is
  declared before it; a `let x;` only it reads is declared where it's made.
- **Two checks no program reaches**: a `try` whose body returns, and one
  whose `catch` drops another value last before this one's end. Lowering
  makes neither (it wraps no `return`, and a crossing value's drop is
  wrapped itself); each stays, so a `finally` never drops twice if that
  changes. They have no mutation, as nothing reaches them.

## Why

- **It's the code a person writes**, and it reads in one place what a
  scope cleans up. 97 corpus programs lost about 6,300 lines.
- **It's the program Rust runs**: every corpus case still matches native
  Rust, the panicking ones (`drop_on_panic`, `drop_index_panic`,
  `drop_replace_panics`, `folded_call_panic_drops`) too, and
  `scoped_drops_cross` holds two crossing lives and a panic after.

## Alternatives

- **`using` declarations**, `[Symbol.dispose]`: the newest idiom, but not
  in every engine rust-js targets.
- **One `finally` that drops a drop's flags**: also covers crossing lives,
  at the cost of a flag per value a reader has to follow.

## Amendment: a loop's drops at each way out are one `finally` (rd191)

A `for` over a `RefCell`'s borrow, `for child in
self.children.borrow().iter().rev() { if f(child) { return true; } }`,
holds the borrow as a temporary of the loop, which MIR drops before each
way out of it, `$unborrow(borrow); break;`, `$unborrow(borrow); return
result;`, and in a `catch` of each step that may throw: a `try` around
`$next(iter)`, around `f(child)`, each, and a `while (true)` no `for`
could be made of. `drop scopes` takes a run its drop ends, which a loop's
drops at its ways out are not. **A `while (true)` loop that drops one
value before each way out of it, its `return`s, `throw`s and own
`break`s, and in each `catch` of what may throw in it, with the `try`s
before it that drop it so, is one `try { .. } finally { drop }`, each of
those drops gone; its loop is then made a `for` of what it steps, as
`loops` makes one, and the `break` that left it, now at the `try`'s end,
goes** (`unwinds::looped_drops`, the "looped drops" pass, before "drop
scopes"). The borrow stays: a callback that borrows the cell mutably
meanwhile panics, as Rust's does. Not of a loop with a way out that
doesn't drop it, nor a labeled jump, nor a statement that may throw
outside such a `catch`: a value whose scope is the loop is its header's
temporary, which no way out moves, so none of those is MIR's of such a
loop, and none is taken. Case A: the same drops at the same times. Held
by `test/corpus/refcell_borrow_loops.rs` against native Rust, 8 corpus
cases that read so, and the mutations `looped-drops-never` and
`looped-drops-break-kept` (`mir/tidy/unwinds.ts`).

## Amendment: a loop's drops read before MIR's blocks are (2026-10-29, rd200)

React Float's `hydrateHoistable` loops over a range of a `RefCell`'s
borrow, which it changes, `for i in 0..nodes.len() { .. nodes.remove(i)
.. }`. "looped drops" ran in the late `scopes` stage, after `jumps`,
`values` and `loops` had read the loop with a `catch` of the drop around
each step: no `for` of the range, `while ((i = $rangeNext(iter)) !=
null)`, a value's `if`/`else` not folded into its `||`, and a block MIR
made kept, `bb93:`. The same function without the borrow reads as
written. **The loop's drops are read first too, in the "drop scopes"
stage right after "flags", before `jumps`** (ADR 0659, amended), of a loop
whose first step is one `loops` makes a `for` of (`loops::step_of`), as
MIR leaves it: its own label, `bb5: while (true)`, its `break bb5` a way
out and `continue bb5` a turn's; a block it labels, `bb25:`, whose
`break`s go on in it, no way out; what follows a block that never falls
out never runs, so it goes. What can't throw before it and only it reads,
the range's `iter = { start: 0, end: nodes.length }`, and the `let`s only
its run reads, are in the `try`, by the one rule `scoped_drops` has
(`gathered_lets`). The late pass stays for the loops a stage after reads
so. A `try` ending the function is its end too, so its body's last
`return;` MIR made goes, as `tail returns` drops the function's own
(`jumps::ending_returns`). Case A: the same drops at the same times, the
corpus against native Rust, `refcell_borrow_loops.rs`'s `take_first` the
React shape; 7 corpus cases read so, `refused_drops_and_displays.rs`'s
`catch` around each step of its loop now one `try`. Held by the mutations
`drop-scopes-stage-empty` (`mir/tidy/stages.ts`),
`early-looped-drops-of-any-loop`, `looped-drops-own-label-unread`,
`scope-lets-left-before` (`mir/tidy/unwinds.ts`) and
`ending-try-return-kept` (`mir/tidy/jumps.ts`). `docs/readability.txt`'s
renamed share went from 654.2 to 654.6 in 10,000 lines, blessed: no name
is renamed that wasn't, the cases are fewer lines.
