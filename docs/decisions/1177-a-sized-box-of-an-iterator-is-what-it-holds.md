# 1177. A sized box of an iterator is what it holds

Status: Accepted (2026-11-21, the lead's decision for #2219's family).
Amends [0465](0465-dropping-an-adapter-is-closing-it.md) and
[1090](1090-an-iterator-conformance-matrix.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): R. The results change:
drops ran twice or not at all, a hint was never asked, and a value was
never stepped or threw.

## Context

In JS a sized `Box<I>` is `I`'s own value. Each question of an iterator
decided this on its own, though. Some peeled a box by hand and some looked
for a type parameter, `ty::Param`, without peeling one. #2219 fixed one of
them, `closing_source`, and the tables had no source that would find the
others. Three sources were added to the iterator matrix and the
conformance table: a sized `Box` of the crate's iterator, and generic
code's `Box<I>` and `&mut Box<I>` of it. Together they failed 6,541
conformance and 14,144 matrix case-builds, in four clusters with one
cause:

- A generic `&mut Box<I>` was given the crate's value raw. The argument
  loop (stepping.rs) peeled the `&mut` and no further, so it was never
  stepped, counted wrong, and its `next` threw (#2221).
- A generic `Box<I>`'s hint was never asked. `hint_reaches_own_where`
  (lengths.rs) unboxed only to look for a `dyn`.
- A generic `Box<I>` made in place was never closed, a leak.
  `closes_to_drop` (closing.rs) asked `has_generic_drop` and
  `is_user_iterator` of the box itself.
- An adapter of generic code's `&mut I` was dropped as std's struct,
  field by field, where it was a JS form made in place. `closing_source`
  answered nothing of a `&mut` of a type parameter. The JS read
  `flatMap.inner` of a `$flatMap` and threw, or never dropped a chain's
  other side.

## Decision

**`held_unboxed` (recognition/held.rs) is the one answer of what a sized
`Box<I>` is: what it holds, each box peeled.** It never peels a reference,
which lends what it's to, nor a `Box<dyn Iterator>`, which carries its own
steps and drop (ADR 0521), nor what's unsized.

The base questions ask it:

- `closing_source`: a box is closed as what it holds.
- `hint_reaches_own_where`: a box's hint is what it holds's.
- `closes_to_drop`: a box drops what it holds.
- `iterator_given_as_it_is`: replaces its own box arm.
- The argument loop: through a `&mut`, then through the box.

Two more rules go with it:

- A `&mut` source holds nothing that closing an adapter drops. Its
  lender drops it.
- A sized box's lend, `$lent(it, "boxed")`, owns what it holds. So
  `closing` closes what it holds by the box's drop and lends it as the
  box again, `$lent($closing(it, drop), "boxed")`. Before, it closed the
  lend, whose drop was then given the lend rather than the value.

`iterator_held`, which peels any box and any reference to reach the impl
a type's steps reach, moves to the same module. A box's own questions
stay where they are: what std's `Box<I>` passes on (`Wrapper::passes`,
`wrapper_lends`), a boxed slice, and a box of std's compiled struct.

test/architecture-iterators.test.ts lists every `boxed_ty()` in the
iterator owners. A new peel fails it until it's named there with its
reason, or asks `held_unboxed`.

## Consequences

- Corpus `generic_box_of_an_iterator_is_what_it_holds` covers both builds:
  a `&mut Box<I>` skipped and counted, stepped, and summed; its `skip`
  and `take` collected; a `Box<I>`'s `step_by` collected into a
  `HashSet`; its `zip(..).nth(1)`; and a `&mut I`'s flatten summed into a
  type with its own `Sum`, and its chain `try_fold`ed.
- Mutations `held-unboxed-peels-nothing`, `hint-reach-box-unpeeled`,
  `closes-to-drop-box-unpeeled`, `box-lend-closed-outside`,
  `argument-mut-box-unpeeled` and `mut-source-closed-as-held` are new.
  `sized-box-source-unclosed`, `box-param-not-held-as-it-is`,
  `box-param-not-given-a-js-iterator`, `shared-ref-iterator-unconverted`,
  `own-collect-unclosed` and `parameter-collect-unclosed` are retargeted.
  `box-iterator-not-held` moves to held.rs.
- `consumer_closes` (closing.rs) asks it too, so a fold of generic code's
  `Box<I>` closes as one of `I` does. No mutation of it is kept: the only
  case it reaches, a generic `Box<I>` folded by a closure that holds a
  destructor, is refused first by the drop check (ADR 0190). The
  diagnostics test pins that refusal.
- The tables grow by the three sources: matrix 163,711 to 191,263 cases,
  conformance 41,742 to 52,557, points 13,254 to 15,969. Known rows are
  remapped by case id. The per-push slices grow by 531 matrix and 240
  conformance cases, a few seconds each.
- What's left, 242 case-builds, is listed under its issues:
  - #2225: a box's own `last` behind std's struct, dropped again.
  - #2227: a `&mut Box<I>`'s enumerate never asks the crate's `nth`.
    Seeing a type parameter through the box in `extension_param` fixes
    it, now that #2226 is fixed.
  - #2224: a box's filter holding a dropping closure, counted.
- #2226's 71 rows, a collect of a map over a `&mut I` made an array's
  `$iter`, went with its fix. A `&mut Box<I>`'s inspect collected where a
  capacity is read is now refused, where it threw (#2230, P2).
- Generic code's `Box<J>` stepped from its back stays refused, as it was.
