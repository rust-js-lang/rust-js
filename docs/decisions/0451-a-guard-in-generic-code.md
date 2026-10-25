# 0451. A guard in generic code: its `Deref` a dictionary's, its drop given, what's read before it

Status: Accepted. Extends [0098](0098-destructors.md), [0328](0328-refcell-borrow-counts.md),
[0394](0394-a-deref-bound.md) and [0397](0397-a-refcell-of-a-dyn.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): N for a guard's
`deref` (1); results would change for its drop and what's read before
one (2, 3), wrong answers that are Rust's now; and a compile error where
Rust's can't be kept (4).

## Context

stable_deref_trait, which url's yoke takes, marks the types whose target
stays put as its `StableDeref: Deref`, a `Ref<T>` among them, so its
impl's dictionary holds std's `Deref` of a `Ref<T>`: refused, as rust-js
had no function for a guard's `deref`. Making one turned up three wrong
answers in generic code given a guard or a value with a destructor:

- A guard given to generic code, `take(cell.borrow())`, was never dropped
  there, so its cell stayed borrowed and a later `borrow_mut()` panicked.
  Which type parameters are given a drop was asked of user destructors
  alone (`holds_user_drop`), where the lowering asks `drop_runs`, which
  knows a guard's, a counted `Rc`'s and a channel's too.
- `d.len()` of a `D: Deref` returned was read after `d` was dropped: a
  read through a shared reference was kept to be read where it's used, as
  nothing changes what one refers to while it's borrowed, but a destructor
  may once the borrow's over, and `0` was returned where Rust's is `3`.
- A `RefCell<dyn T>` is its pair, `{ value, impl }` (ADR 0397), but
  generic code over a `RefCell<T: ?Sized>` reads it as a cell of a value,
  and JS threw.

## Decision

1. **A guard's `deref`, in a dictionary, is its value**, `(x) => x.value`,
   by the one function that says what a guard derefs to, `guard_value`,
   which its call asks too: `$refStableDeref = { Deref: () => ({ deref:
   (x) => x.value }), $drop: $unborrow }`.
2. **A type parameter is given a drop where a caller gives it a value whose
   drop runs something, by `drop_runs`**, the lowering's answer: a guard,
   a counted `Rc`, a channel's end, a destructor. A crate none of whose
   values' drop runs anything gives none, as before.
3. **A value kept to be read in place that reads through a shared
   reference is read before a drop**, assigned there, its `let` first in
   the function: `result = tmp.length;` inside the `try` whose `finally`
   drops `d`. Not an element or an object made where it's used, which
   reads what a branch after sets.
4. **A cell of a `dyn` given to generic code as a cell of a type
   parameter is refused**, where the code's signature or its impl's type
   holds that parameter in a `RefCell`, a lock or a guard of one, as a
   `dyn` given for a type's fact is (ADR 0145).

## Why

- **One place answers each question**: what a guard derefs to, asked by
  its call and its dictionary; whether a value's drop runs something,
  asked by the lowering and the analysis that gives generic code its
  drops.
- **Rust's answer, or a compile error**: each wrong answer is Rust's now,
  or refused where it can't be.
- **It's tested**: a corpus case gives a dictionary of std's `Deref` of a
  `Ref`, a `RefMut`, a `Vec`, a `String` and an `Rc`; another gives
  generic code a guard and a value whose destructor changes what it's
  read through, against native Rust; the refusal is in the limitations; a
  mutation of each part is caught.

## Consequences

- url gets past stable_deref_trait's `Ref<T>`, to its `&mut T`'s `Deref`:
  a `&mut` given as a type parameter's value is the object, and generic
  code over `&mut T` takes a box (ADR 0180), a representation question
  of its own.

## Amendment: only what's read past the drop

Decision 3 read before a drop every value kept to be read in place that
reads through a reference, those a branch was done with too: a map's
`len()`, read in a loop's branch, was read again before a drop after
the loop, where the map wasn't declared, and JS threw (a generated
program's, seed 12446640, failing the nightly run since). Now it reads
only those `read_from` the drop's successors, before they're assigned
again. The `kept_read_past_branch` corpus case reads one in a loop's
branch, drops a value each pass and one after the loop, against native
Rust.

## Amendment: read before the branches, not in the one that drops

A value kept to be read in place lives in one map for the whole body, so
settling it before a drop in one branch removed it for the branches
lowered after. Past where they met, those read a variable only the
dropping branch set: `*e > match k { 0 => 1, 115..=967 => D(..).0, _ => 0 }`
compared `undefined` in every arm but the one that drops `D`, and a
`filter` kept nothing (#274, fuzz seed 50786). Following ADR 0262's
"results change" case, Rust's answer is kept: before a block branches,
each such value that a drop it dominates would settle is read there,
where every branch has it. The drop's own settling still serves a drop
with no branch before it. The `read_past_dropping_arm` corpus case
compares a reference's value past a `match` with a dropping arm, in a
function and a `filter`, against native Rust. A mutation of the new
settling is caught. A narrower form, only drops before the branches
meet, changed no output in the corpus, so the rule doesn't have it.
