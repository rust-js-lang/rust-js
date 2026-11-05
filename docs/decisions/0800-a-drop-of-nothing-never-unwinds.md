# 0800. A drop of what JS drops nothing of never unwinds

Status: Accepted. Amends [0364](0364-mir-lowering.md), whose drops are
elaborated as rustc's are, as rustc finds of a drop of what isn't there.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, it decides no
disagreement: a drop that runs nothing can't panic, in Rust as in JS, so
no program can tell; only where a temporary is held changes.

## Context

React's `enqueueTask` (shared/enqueueTask.js) gives a `MessageChannel`'s
port a handler:

```js
channel.port1.onmessage = () => {
  callback();
};
```

Its port, in the `shared` crate, a library React's other packages use,
printed

```js
const value = () => {
  callback();
};
channel.port1.onmessage = value;
```

`set_onmessage(Some(Box::new(move |_| callback())))` is, in MIR, a
`Box<closure>` made, coerced to a `Box<dyn FnMut(&MessageEvent)>`, and
dropped, `drop(_8)`, which unwinds to a cleanup that drops the `Box<dyn
FnMut>`, `drop(_7)`. In a library, what a `dyn FnMut` holds may have a
destructor a consumer gives it, so the function's drops are elaborated
(0364). The `Box<closure>` has moved, and its closure drops nothing in JS,
so its drop runs nothing; but the dataflow that says what each drop finds
there only tracks what drops something, so it took the box's drop to
unwind, and the `Box<dyn FnMut>` to be there to drop in the cleanup. A
drop reads what it drops, so the box was read more than once, held in a
`const`. In an app, where a `dyn FnMut` of the crate's closures drops
nothing, no drop is elaborated, and the closure was written where it's
given. Every callback the closure holds, a `&'static dyn Fn()`, a `fn()`,
a number or nothing, was held alike; the receiver mattered only as a
getter's read, `channel.port1`, past which tidy moves nothing held.

rustc's own elaboration (`compute_dead_unwinds`) removes the unwind of a
drop of what isn't there before it asks what each cleanup drops.

## Decision

**A `Drop` of what JS drops nothing of runs nothing, so it never unwinds.**
The dataflow of what each drop drops runs on the body without the unwind
of each such `Drop`, as rustc's elaboration runs without the unwind of a
drop of what isn't there. What only such an unwind reaches drops nothing,
and what's made before it isn't read by that cleanup.

`channel.port1.onmessage = () => { callback(); };` again, in a library as
in an app, of every callback.

## Why

- **The cleanup is never reached in JS**: the drop it would unwind from
  runs no code, so nothing there can throw. Counting the cleanup's reads
  kept a temporary for a drop that never happens.
- **It's rustc's rule, for JS's drops**: rustc removes the unwind of a
  drop of nothing there; a drop of what drops nothing in JS is one too.

## Alternatives

- **Count only the reads of drops a cleanup reached by a call makes**:
  another walk of the cleanups, and the flags those drops set would still
  be decided by the dataflow that took the box's drop to unwind.
- **Track every type in the dataflow, as rustc's `needs_drop` does**: a
  `Box<closure>`'s drop would be known to be of nothing there, but every
  other path rust-js drops nothing of would be tracked too, for no drop.

## Consequences

- A function whose drops are elaborated holds no temporary for the
  cleanup of a drop that runs nothing; its other drops, and their flags,
  are as they were. `rc_counts.rs`'s first `leaf.parent.borrow().upgrade()`
  was held as `node`, a value a cleanup it can't reach dropped, named for
  what it drops; it's `tmp` now, as its two others were.
- Tested by `test/lowering-moves.test.ts`, a library's setter given a
  closure of each callback, and by the corpus's drop cases.

## Amendment: calls that unwind to the same drops share one cleanup (2026-11-05, #723)

`drops::cleans_up`, the cleanup a call's panic unwinds to, answered with
the cleanup chain's first block, even where that block and those after it
drop nothing, a temporary each call moved on, and write nothing. So
consecutive calls whose chains differ only by those blocks,
`v.into_iter()`, its `enumerate()` and its `rev()` beside a live `D`,
seemed to unwind to different cleanups. At each one, structure.rs made
what was made before it a `const`, held apart from the call that reads
it. A `rev()` then found `var _4`, no form of the enumeration left to read,
and refused it. A peeked `peekable()` counted earlier in the function
made the same happen, so the refusal depended on the order of unrelated
statements (#723). The answer is now the first block of the chain that
drops or writes something: one cleanup for calls that drop the same, and
what's made between them stays made in place.

Case N (ADR 0262): which block the cleanup starts at runs the same drops,
so no program can tell. Only where temporaries are held changes. Of the
corpus, 110 snapshots read differently: mostly a `let tmp; try { tmp = ds(); }
catch ..` per call merged into one `try`, 1,410 lines fewer in all. Some
read worse, filed as #769: a temporary named after its type, `l` or `d`,
where it was named after its call, `next` or `nthBack`; a closure
declared as `function map(d)` now `let map; .. map = (d) => ..` in a
`try`; `$dispose($removeAt(handlers, 0))` now through a `const value`.

The readability gate's shares of `$iter` (89.4 to 89.9 in 10,000 lines),
`Iterator.from` (8.8 to 8.9) and `return;` (12.8 to 12.9) grew with it
by arithmetic alone, blessed after: over the files the baseline lists,
`$iter`'s count fell from 1,677 to 1,671, and `Iterator.from`'s, 165, and
`return;`'s, 239, stayed, as the lines they're shares of fell from 187,439
to 185,955, the `try`s merged.

It also showed a lowering that read its argument in place past an effect,
`Option::insert`'s drop of the old value, right only where such a spurious
flush held the argument first (#765, fixed before this).

`back_steps_beside_a_live_drop` holds #723's shapes against native Rust
in both builds: a `Vec`'s, an array's, a slice's and a range's
`enumerate()`, `skip()` and `take()` stepped from the back, beside a live
`D` and after a peeked count, and a panic through them. Mutation
`cleanup-starts-at-its-chain` (`lower/mir/drops.ts`).
