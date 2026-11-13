# 0451. A guard in generic code: its `Deref` a dictionary's, its drop given, what's read before it

Status: Accepted; amended (what's read through a guard whose drop writes nothing, #452). Extends [0098](0098-destructors.md), [0328](0328-refcell-borrow-counts.md),
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

## Amendment: at the first test, once it's made, past a drop JS sees

The settling before the branches ran before a block's test was made, so
what `flush` keeps in place only as the test is made wasn't there to
settle, and the next test in settled it instead. In React's
`mapIntoArray`, `escapedPrefix.to_string() + &(if key.is_some() && (!c ||
other) { .. } else { String::new() }) + &child_key` read `escapedPrefix`
inside the first test's branch: past a missing key it was `undefined`,
and the key `"undefined" + childKey`, which failed seven of
ReactChildren's tests. Following ADR 0262's "results change" case,
Rust's answer is kept: the value is settled at the switch, once its test
is made and before its cases are, at the first test the value is there
for, which every way past it goes through. And only a drop that drops
what JS sees (the drop elaboration's `drops`) settles: a `String`'s,
the only drops there, settle nothing, and the `if` reads as the one
expression it was before ADR 0451's first amendment, `escapedPrefix +
(mappedChild.key != null && (..) ? .. : '') + childKey`. The
`read_before_short_circuit` corpus case joins a borrowed `&str`'s
`to_string()` to such an `if`, of a `String` dropped in its branch and of
a `Drop` type, on every way through its tests, against native Rust; with
the old placement it printed `undefined.`. Mutations settling past any
drop and skipping the settling are caught, and `kept_read_past_branch`
now holds what the playground held for a read in a branch that's done
with, which isn't settled again past a later test, `root.slice(0, -3);`.

## Amendment: what's read through a guard whose drop writes nothing (#452)

A P0, a wrong answer. Case ([0262](0262-when-rust-and-js-disagree.md)):
results change, preserve Rust's. `let Some(chunk) =
handler.chunk.borrow().clone() else { return 0.0; }; touch(handler);
chunk.id`, of a field `RefCell<Option<Rc<Chunk>>>` tracked at run time,
printed `const borrow = $borrow($fieldCell(handler, "chunk")); ..
touch(handler); return borrow.value.id;`. The `Rc`'s clone is the value
(`copies.rs`), read in place as what reads the same anywhere, since a
guard, `Ref<T>`, is plain data while it lives. But its drop, which writes
nothing where nothing counts the cell, settled nothing: only a drop that
prints something did (`settle_reads`). So the value was read after
`touch` had emptied the cell. Native Rust prints 5; the JS threw a
TypeError.

**A guard's drop that writes nothing settles each value kept in place
that reads through that guard and is read past it**
(`temporaries::settle_guard_reads`, from `structure.rs`'s `Drop`). Values
read through any other reference keep waiting for a drop that writes
something, as before: settling them at a quiet drop put the assignment
in one branch where another reads the variable
(`returns_joined_past_blocks`, `root ${parent.tag}` of a `parent` never
set). So the clone is read where Rust makes it: `const chunk =
borrow.value; touch(handler); return chunk.id;`.

Held by the corpus's `guard_value_read_before_write` against native Rust,
a temporary guard and a named one dropped by `drop`, and
`let_else_guard_released`'s `const y = value.value;`. Mutations
`guard-drop-reads-unsettled` (`structure.rs`) and
`guard-settle-any-reference` (`temporaries.rs`). A sweep of the corpus and
snapshots for a guard read past its `$unborrow` finds only reads right
after it, with nothing that could write the cell in between.

Not reproduced here: the Flight client's `const {value: _12} =
$borrow($fieldCell(handler, 'chunk', true)); const chunk = _12;`, which
reads the value right away (correct) but leaks MIR's name. That's a
counted guard's form, another cause, left for naming.

## Amendment: what's pending before a loop is made before it (#1395)

A value kept to be made where it's read (`mir/temporaries.rs`' pending
values), the loop's header reached right after it, was flushed into
nothing as the loop began: `flush` writes a value not read the same
anywhere as a `const` and marks its name declared, and the header's call
of it was given a list it dropped. `pair(200 - 1, loop { k += 1; if k ==
3 { break k; } })`, a call's argument made before a later argument that's
a `loop`, read the name in the `break`, a `ReferenceError` where Rust
prints `(199, 3)`, in both builds; Rustlantis's seeds 1909 and 2471 were
it of custom MIR. What's flushed at a loop's header is now written before
the loop, where Rust makes it: `const value = 200 - 1;` then `while
(true)`. Case C of ADR 0262.

## Amendment: a consumer's drop-free shortcut asks its libraries too (#1591)

Case ([0262](0262-when-rust-and-js-disagree.md)): R, results change, so
Rust's behavior is kept.

Decision 2's "a crate none of whose values' drop runs anything gives
none" held of the crate's own values alone. A consumer compiles a
library's generic impls' methods itself, `Bag`'s `Extend::extend`, and the
library's own generic code calls them through the dictionary the consumer
gives, `up(Bag(vec![]))` calling `c.extend(Up { n: 0 })`, with the
library's values. A consumer none of whose own values drops gave that
method no drop of `T::IntoIter`, so its `for` loop never dropped `Up`:
`drop Up 3` never printed (P0, both builds; found by the metamorphic
net's `library` rewrite of #1511's case, which moves every item but `main`
into a library).

Now:
- A library's contract says whether any of its own values drops,
  `Library.drops_any`, `drops.rs`' own answer, `drops_any`, the one
  decision (ADR 1129; contract version 32). A manifest without it reads
  as `true`, never as drop-free.
- A consumer takes the shortcut only where none of its own values drops
  and no library under it, each of which it's told of, says one does
  (`Foreign::libraries_drop`). A consumer of drop-free libraries is as
  it was: no corpus case's JS, no crate pair's and no crate of the crate
  corpus changes.

Crate pair `library_extend_drops_the_crates_iterator`, a library's own
iterator given to its own `Bag` by its generic code, and triple
`library_extend_given_a_middle_crates_iterator`, a drop-free library's
`Bag` extended by a middle library's generic code with its `Up`, each
against native Rust. Mutations `libraries-drop-unasked` (`drops.ts`),
`drops-any-unexported` (`library/exports.ts`) and
`drops-any-missing-drop-free` (`library.ts`, by the unit test
`a_contract_without_drops_any_may_drop`), caught;
`drops-without-destructors` and `library-drops-skipped` refitted.

## Amendment: an impl's items' iterator is dropped by one route, its bound's dictionary (2026-11-13)

Case ([0262](0262-when-rust-and-js-disagree.md)): none, what's printed is
native's either way.

An impl's method whose where clause bounds its `Self::Item`, `<Wrapper<I>
as Iterator>::Item: IntoIterator`, drops what each of its items' iterators
holds. It had two routes to that drop: the clause's self type normalized
to the impl's `<I as Iterator>::Item`, a slot of the impl's, which gave the
impl two more drop parameters, `dropIItemItem` and `dropIItemIntoIter`
(#1080); and the bound's own dictionary's entry, `$dropIntoIter`, which
`item_drop` reads (ADR 0178). The mutation that took the first route out,
`bound-self-unnormalized`, survived: every case dropped the same by the
second. The first is removed (`drop_items_of` takes the clause's self type
as it's named), so the dictionary's `$dropIntoIter` is the one route,
`WrapperItemIntoIterator.$dropIntoIter?.(held.iter)`, and the impl takes
the drops of its own parameters alone.

`impl_method_drops_its_items_iterator.rs` and
`default_of_a_non_blanket_impl_steps_its_self.rs`, re-blessed, and the
crate pair `library_impl_method_where_clause` print native's drops, in
both builds. Mutation `item-drop-of-a-dictionary-unread`, the dictionary's
drop unread, is caught by the first; `bound-self-unnormalized` is removed
with its route.

The survivor `library-params-never-drop` is caught by crate pair
`library_without_destructors_drops_what_a_pointer_holds`: a library with no
destructor of its own whose generic code holds an associated type's value,
reached through a function pointer it gives out, which its consumer can't
compile its own of, so the library's own JS drops it (ADR 0163).

## Amendment (2026-11-13, #1650): a destructor made only in generic code still drops

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior is kept. Whether any of a crate's values drops something,
`drops_any`, which gives generic code no drops where none does, asked only
of types with no type parameter. A crate whose only values with a
destructor are made in generic code, `Token<I>` of a `Drop` impl for
`Token<T>` in `pairs<I>`, was taken to drop nothing, so a `Chain`'s
`parse(self, ..)` that returned early by `?` dropped the `Token` its `self.1`
held never. Now a value of a type of generic code counts where it holds a
struct or an enum of the program's own with a destructor, which runs
whatever its parameters are. No snapshot moves. Corpus case
generic_struct_with_a_destructor_dropped_on_an_early_return against native
Rust; mutation `generic-destructors-unseen` (`analysis/drops.ts`).
