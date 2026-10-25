# 0907. A fn pointer runs what the crate made it of

Status: Accepted. Extends [0580](0580-a-call-asks-of-a-cell-only-what-it-may-borrow.md)
(what a call may borrow) to a call of a fn pointer, which ran "what
rust-js can't see".

Case ([0262](0262-when-rust-and-js-disagree.md)): A. A borrow left
uncounted is one no other borrow of its cell can meet: a fn pointer only
ever holds a function the crate's code made one of, each of which is seen
to borrow nothing that may be the cell held. Where a value of its type may
come from outside the crate's code, it's anything, as before, and its
borrows are counted and panic where Rust's do.

## Context

React's ReactFiberLane walks a root's lanes while it holds its tables:

```js
export function getEntangledLanes(root, renderLanes) {
  ..
    const entanglements = root.entanglements;
    let lanes = entangledLanes & allEntangledLanes;
    while (lanes > 0) {
      const index = pickArbitraryLaneIndex(lanes);
      ..
      entangledLanes |= entanglements[index];
```

and `pickArbitraryLaneIndex` counts leading zeros through `clz32`, which
is `Math.clz32` where the host has it and React's fallback where it
doesn't. Its port is a thread-local fn pointer:

```rust
thread_local! {
    pub static CLZ32: fn(u32) -> u32 = if js::is_defined(u32::leading_zeros) {
        u32::leading_zeros
    } else {
        clz32_fallback
    };
}

fn pick_arbitrary_lane_index(lanes: Lanes) -> usize {
    31 - CLZ32.with(|clz32| clz32(lanes as u32)) as usize
}
```

A call of a fn pointer was one of code rust-js can't see (ADR 0580), so
`pick_arbitrary_lane_index` might borrow anything, and every borrow held
across it was counted: `$borrow(root.entanglements, true)` and a `try {
.. } finally { $unborrow(..) }` in `getEntangledLanes`, `$borrowMut(..)`
in `markStarvedLanesAsExpired`, and in `markRootFinished`, which holds
three, a `try`/`catch` around each call that undid each borrow live, and
its `for` a `$iter`/`$next` loop, each step of which may throw.

Counting is by type (ADR 0863): once one borrow of a `RefCell<Vec<i32>>`
is held while code that may borrow one runs, every borrow of that type
is checked, since the code run may be any of them. So the one fn pointer
counted every `LaneMap` of the root's: `entanglements` of `Lanes`,
`expirationTimes` of `f64`, `hiddenUpdates`, `pendingUpdatersLaneMap` and
`transitionLanes` too.

## Decision

**A fn pointer's call runs what the pointer may be** (`analysis/fn_pointers.rs`,
`FnPointers::may_be`): each function and closure the crate's code makes a
fn pointer of that type of, by a coercion anywhere in its bodies, which
rustc's typeck records, the initializers of its statics and thread-locals
among them; or anything, where a value of its type may come from outside
the crate's code. What it may be is asked as a call of it is: a function
of the crate's runs what its summary says, std's what its bounds do, a
binding what its marks say (ADRs 0867, 0898), and a closure its body.
Both the crate's summaries (`direct`) and a guard's extent (`Quiet`, ADR
0328) ask it, through `CellBorrows::pointer_call_asks`.

A fn pointer is made only by a coercion, of a function or of a closure
that captures nothing. One the crate didn't make reaches its code from
outside it only by what crosses into it, so a type is *anything* where a
value that may hold one of its fn pointers, through what it's made of and
the fields of a type that isn't std's:

- is given to, or by, a library's function or a binding, which may make
  one of any function, give one, or change one it's given what holds;
- is a library's static or constant, or a library's `Deref` gives it;
- is a parameter or the result of a function JS or a consumer may call,
  an exported one or a trait impl's (`library::reachable`);
- is what std gives of what it isn't given, by addressing memory or a
  downcast of a `dyn Any` (`recognition::gives_what_it_isnt_given`): any
  other std function gives only what it's given, and an empty `Vec<T>`
  holds no `T`;
- is the result of a generic function of the crate's whose signature's
  result names a type parameter, which may be one it has from outside.

Its type is one whatever its lifetimes: `for<'a> fn(&'a Root)` and
`fn(&'static Root)` are one. One whose signature holds a fn pointer or a
`dyn` of its own, whose binders aren't compared, is anything.

```rust
pub fn entangled_lanes(root: &Root, mut lanes: i32) -> i32 {
    let entanglements = root.entanglements.borrow();
    let mut entangled = lanes;
    while lanes > 0 {
        let index = pick_arbitrary_lane_index(lanes);
        entangled |= entanglements[index];
        lanes &= !(1 << index);
    }
    entangled
}
```

```js
export function entangledLanes(root, lanes) {
  const entanglements = root.entanglements;
  let entangled = lanes;
  while (lanes > 0) {
    const index = pickArbitraryLaneIndex(lanes);
    entangled |= $index(entanglements, index);
    lanes &= ~(1 << index);
  }
  return entangled;
}
```

## Why

- **The check it drops can't fail**: what the call runs is one of the
  functions the crate made the pointer of, each seen to borrow nothing
  that may be the cell held.
- **Rust's own closed world**: a fn pointer is a function some coercion
  made one of. The crate's are all in its bodies; the others come in only
  through what crosses its edge, which the types say.
- **Upstream's JS**: `const entanglements = root.entanglements`, a table
  read with no count kept of a borrow no other can meet.

## Alternatives

- **Follow the value**: which fn pointer a call reads, `CLZ32`'s, and
  what its initializer may give. Exact here, but a flow analysis of every
  place a fn pointer passes through, where a type answers each the same.
  Rejected.
- **Count per borrow, not per type**: only a borrow held while code that
  may borrow its cell runs keeps a count; every other one of the type
  still asks whether one is held, since the code run while that one is
  held may be any of them. So the others would still be a check each,
  `$borrow(..)`'s or another of its own. The type's count stays (ADR
  0863); this decision is what took the `LaneMap`s out, by the fn pointer
  no longer running anything.
- **Trust every fn pointer to run only the crate's functions**: a JS
  function JS gives an exported function, which borrows the cell held,
  would pass where Rust's panics. Rejected.

## Consequences

- No corpus case's JS changes but the new ones. React's ReactFiberLane
  has no `$borrow`, `$borrowMut`, `$unborrow`, `$iter` or `$next` left.
- A fn pointer of a type JS or a library may give, of an exported
  function's parameter say, is anything: a borrow held across a call of
  one is counted, of every `RefCell` of its type.
- A type's fn pointers are each the crate made one of anywhere, not only
  those that reach the call: a pointer of the same type that does borrow
  the cell, made elsewhere, counts it.

## What holds it

- `test/lowering-cell-borrows.test.ts`, "a borrow held while a fn pointer
  runs what the crate made it of isn't counted": `clz32` of
  `Math.clz32` or its fallback, its table the array; and a fn pointer JS
  gives an exported function, counted, which throws `already borrowed`
  where it borrows the cell.
- The corpus's `fn_pointer_borrows_nothing`, `fn_pointer_borrows_held`
  and `fn_pointer_closure_borrows_held`, against native Rust: a fn
  pointer of a function or a closure that borrows the cell held panics.
- Mutations in `scripts/mutations/lower/analysis/fn_pointers.ts`:
  `fn-pointer-made-of-nothing`, `fn-pointer-closures-made-of-nothing`,
  `fn-pointer-never-from-outside`; in `cell_borrows.ts`,
  `fn-pointer-call-runs-anything`; in `drops/facts.ts`,
  `pointer-call-beside-guard-asks-nothing`.
