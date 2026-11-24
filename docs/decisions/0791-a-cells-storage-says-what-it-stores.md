# 0791. A `Cell`'s storage says what it stores, however it's reached

Status: Accepted. Amends [0772](0772-a-nullable-cell-field-writes-null.md),
whose `&Cell` given elsewhere wrote `undefined`, and whose call sites were
told by THIR's spans; and [0099](0099-mut-references.md)'s handle, which reads
its place each time and now reaches it once (#191). Amended by
[0841](0841-a-nullable-place-stores-what-is-never-undefined-as-it-is.md):
what's never `undefined` is stored as it is.

Case ([0262](0262-when-rust-and-js-disagree.md)): A for the nullable
field, `null` and `undefined` alike `None` to the program, `null` kept as
the field makes it; C for a place a call reaches, whose call Rust makes
once.

## Context

Review #191 found three things.

- **Facts read again at each call.** Whether a call is a nullable field's
  `Cell`, or a registered symbol's read or `==`, was answered by scanning
  the body's THIR at each call that asked, `registered_equalities(thir)
  .contains(&span)`: four call sites, each building a whole-body set; so
  were a pattern's named `const` at each `switch` case, a component's
  given props at each pattern, and a held guard twice a body.
- **The storage told by the syntax.** ADR 0772 wrote `null` where the
  call's receiver was written as the field, `context.renderer.set(None)`;
  `let r = &context.renderer; r.set(None)`, or a helper given
  `&context.renderer`, wrote `undefined`. And whether what's written is
  `null` or a value already was THIR's `is_some`, beside MIR's `nulled`,
  which answers it for a binding's nullable parameter.
- **A wrong answer, found testing it.** A handle on a place a call
  reaches, `&get(&c).value`, read its place each time:
  `get(&c).value.replace(9)` called `get` twice, and a helper given the
  handle once each read and write. Native Rust calls it once.

## Decision

**A body's facts are its `BodyFacts`, collected once from its own THIR
before it's lowered**: `registered_reads`, `registered_equalities`,
`held_guards`, `present_calls`, `constant_patterns` and the props
`jsx_given_props` finds, beside `named_callbacks` and `kept_casts`. A
closure's are its own body's, as each body's frame is. The queries are
private to `body_queries.rs`, so a call site can't build them again: it
looks its answer up; the architecture test, "a whole body's facts are its
BodyFacts, read once", fails where one's made public again.

**What a `Cell` stores is its storage's, carried by the handle on it.**
A handle is a place and how that place stores what's written,
`ExprKind::Handle(place, js::Storage)`: `AsGiven`, or `NullForNone`, a
nullable field's. `Storage::written(made, nulled)` is the one answer to
what's stored, where a struct is made, a `Cell` set, a nullable function
returns, tidy rewrites a write and a handle's setter is printed; it moved
from `bindings::nullable_written` into `js`, as printing can't ask
lowering. A nullable field's `Cell` (`representation::nullable_cell_field`,
the one answer to which is one) is a `NullForNone` handle. So:

| Rust | JS |
|---|---|
| `context.renderer.set(None)` | `context.renderer = null;` |
| `let r = &context.renderer; r.set(None);` | `context.renderer = null;` |
| `reset(&context.renderer)`, `fn reset(cell: &Cell<..>) { cell.set(None) }` | `reset({ get value() { return context.renderer; }, set value(value) { context.renderer = value ?? null; } })`, `cell.value = undefined;` |
| the same `reset(&context.plain)`, a `Cell` that isn't nullable | its setter `context.plain = value;` |

A write `cell_call` makes (`set`, `replace`, `take`) asks the receiver's
handle; one through a variable that holds the handle, `r.set(None)`,
reborrowed or copied, is `r.value = ..`, which tidy writes as the handle's
storage stores it where it takes the variable to its place
(`read_handles`), and the handle's setter where it doesn't: the handle
is the one carrier of its storage. A first cut also followed such a
variable through MIR to its field, which the handle made redundant; it's
gone.
Whether what's written is `null` or a value already is MIR's `nulled`,
the one answer, given as `Call::nulled`: `set(Some(renderer))` is
`context.renderer = renderer;`.

**A handle written through keeps its place** (#192). Tidy's
`read_handles` takes a `const` handle whose `value` alone is used to its
place; written through, `r.set(5)` in a closure of `let r =
&context.value`, it read the place once where it was made, `const r =
context.value`, and wrote the copy, `r = 5`, a `const`'s assignment that
throws. Now one written through, `js::Writes`' `changed`, is its place at
each use, read and written there, as its storage stores it: `context.value
= 5;` in the closure, `context.renderer = null;` of a nullable field's.

**A std call's kind is chosen of what it reads alone.** Seven kinds of
std call, `std_calls.rs`' handlers, were tried in turn, each given `&mut`
access and the arguments, and one that declined was checked to have
written nothing and taken nothing, by counts that don't prove its state
the same (ADR 0713). Now each handler is `fn(&self, &State, StdCall) ->
Option<StdLowering>`: its `match`, patterns and guards as they were, reads
alone, and an arm that takes the call gives a closure of its work, given
`&mut` access only once it's chosen. The dispatcher asks each, asserts
one took it, and runs it. An arm two kinds both took, `and_then` of a
place found in JS's units, is the iterator kind's, before its other
combinators'. The architecture test, "a std call's kind is chosen of what
it reads alone", fails where a handler takes `&mut` access again.

**A handle reaches its place once.** Where a reference is made of a place
that isn't fixed, `&get(&c).value`, what holds it is named first, `const
item = get(c);`, and the handle is on `item.value`; `replace` and `take`
read and write a place made fixed the same way.

## Consequences

- A `Cell` set once through a call is a `const` more where it was one
  expression: `get(&c).value.set(5)` is `const item = get(c); item.value =
  5;`, as the reference can't tell how often it'll be read.
- Other writes through a nullable field's `Cell` reached as a variable,
  `swap`, `update`, `replace_with`, store what they're given; given to a
  helper, its handle stores `null`.
- Tested by `lowering-nullable.test.ts` (a closure's facts, two calls in
  one body; a local reference, a copied one, a closure's and a helper,
  each of `set`, `replace` and `take`, of a nullable and a plain `Cell`),
  and `cell_reached_through_a_call.rs` and `cell_alias_in_a_closure.rs`
  against native Rust.

## Amendment: a `&mut` to any nullable place (2026-11-04, #541)

The handle that stores as its place does is now the `&mut` to any place that
stores `null` for `None`, a field, an item or a parameter, not only to a
nullable field's cell. It's held by a variable, as a guard's is, since
`Expr::held` of a handle is its place alone. The same goes for what a nullable
thread-local that's its module's variable lends, resolved where its closure is
applied in place ([ADR 1078](1078-a-nullable-place-stores-null-however-its-written.md)).

## Amendment: a handle on a handle stores as the first does (#982)

A `&mut` to a nullable field given whole, `Box::new(&mut fiber.owner)`,
is a handle on the first handle's `value`. Tidy's `read_handles` reads both
as their places in one sweep. It dropped the first handle's `const`, and
then wrote the second's place, `place.value`, where it was used, so the
program threw `place is not defined`. Now a handle on another read handle's
`value` is composed first: it takes that handle's place, and, where it
stores what it's given, that handle's storage. Writing through it is then
the field's own write, `fiber.owner = noneOf(1) ?? null`. The program can't
run otherwise, so Rust's behaviour is kept
([0262](0262-when-rust-and-js-disagree.md)). Held by the corpus case
`nullable_field_borrow_given_whole` and the mutation
`handle-on-a-handle-uncomposed`.

## Amendment: a swap's either side, and a handle given on whole (#1018)

The metamorphic net's `field` rewrite put one side of a write through a
borrow in a wrapper, and the `null` a nullable place stores for `None`
was lost: `mem::swap(&mut x, &mut other.0)` of a nullable parameter
`x`, `[x, other[0]] = [other[0], x]`, wrote `undefined`, so `x`'s `===
null` test read `Some` (native `true true`, JS `true false`), as only a
`take` or a `replace` wrote its one place so; and `let slot =
Held(&mut fiber.0.owner); *slot.0 = None` stored `undefined` in the
nullable field, the struct given a handle on the stored handle's `value`,
`Expr::handle(place.value)`, which tidy made one on the field that stores
as it's given. A swap now writes each side so where it's a handle on such
a place (`unhandled`), `x = other[0] ?? null`; and a handle
`through_handle` holds, given on whole, is that handle (`value_expr`),
which stores as its place does. Results changed
([0262](0262-when-rust-and-js-disagree.md)), so Rust's are kept; no other
case's JS changed. Held by the corpus case `nullable_written_beside_a_wrapper`
and the mutations `swap-into-a-nullable-place-as-given` and
`held-handle-given-on-as-a-handle-on-it`; the net's four rows of #1018
run as native.


## Amendment: one maker of a handle on a place (2026-11-24, #2427, #2426)

A wrong answer (P0, #2427). A `&mut` to a `#[rust_js::nullable]` field
lent on as a handle stored `undefined` for `None`, while a read of a
field only the crate writes tests `=== null`. `dynamic(&mut |o| *o = None,
&mut s.v)` printed `false None` where Rust prints `true None`. Six ways
of lending one on did this: a `&mut dyn FnMut`'s argument, a function
that gives the `&mut` back, a vector that keeps it, an item's field given
back, a `RefCell` borrow's field and a `Box`'s field. Each made its own
handle on the stored handle's `value`, `Expr::handle(place.value)`, and
tidy read that variable's place, so the handle assigned the field what
was written as it was given. `get_or_insert`, `insert` and
`get_or_insert_with` of a nullable field crashed the compiler the same
way (#2426): their `&mut` to the payload was such a handle, and tidy
removed the variable it named.

**A handle on a `&mut`'s place, and a `dyn`'s pair on one, have one
maker**: `mut_refs/handles.rs`'s `handle_on`, `pair_on` and `pair_made`.
A handle a variable holds of a `&mut` to a place that stores its own way
is recorded with its place and its storage (`Made::handles`,
`through_handle`), and `noted.rs`' `held_handle` finds it. The maker gives that handle itself wherever its
`value` is lent on, and a pair on it is on its place and stored as it
stores. Every handle and pair the MIR's lowering makes of a place comes
from the maker: `value_expr`'s, `boxed_args`', a shared borrow's of a
kept `&mut`, an `Option`'s payload's (`option_place_through`) and a
`dyn`'s pair (`unsize_trait`). The architecture test "a handle on a place
is handles.rs's alone in the MIR's lowering" holds that.

Tidy's repair of a pair on a stored handle's `value` (#977) no longer
covers a nullable place; the maker does. It stays for a flattened state's
handle, which only tidy sees as a variable's.

Case ([0262](0262-when-rust-and-js-disagree.md)): results changed, so
Rust's. Held by the corpus case `nullable_field_lent_as_a_handle` against
native Rust: the six ways, the three right before, a `dyn` trait object,
and `get_or_insert`, `insert` and `get_or_insert_with`. Mutations
`held-handle-unread` (`noted.ts`), `handle-on-a-held-handle-made-apart`,
`pair-on-a-held-handle-stored-as-given`,
`pair-made-after-its-handle-renamed` and `reborrowed-pair-place-unfixed`
(`mut_refs/handles.ts`). `held-handle-given-on-as-a-handle-on-it` is
retired: its guard is now the maker's. A flattened state's repair in
tidy, the second place that stores into a handle, is #2429.

## Amendment: a flattened state's handle is held where it's made (2026-11-24, #2429)

Tidy repaired two things on a flattened state's handle (`Storage::InPlace`),
`task.state.get_mut()`'s. A `dyn`'s pair on its `value` became a pair on
its place that sets each key (#977). A handle on its `value` became the
handle itself (#678). Both were a second place that stored into a
handle, because the MIR made each of them as-given, and only tidy knew
the variable held such a handle.

**Each handle a variable holds of a place that stores its own way is now
recorded where the MIR's lowering holds it**, so `mut_refs/handles.rs`
answers for a flattened state as it does for a nullable place:

- `Handled::of` is the rule: any storage but `AsGiven`.
- `Made::note_handle` records it.
- `hold_handle` (noted.rs) is the spill that records it. It's used by
  `through_handle` and by a flattened state's lend in cell.rs:
  `get_mut`, `Cell::from_mut` and `try_borrow_mut`'s guard.
- `note_held_handle` (temporaries.rs) records a `const` the MIR writes
  once with no part borrowed: flush's, and `mir_store`'s. It also
  records such a `const` that's another name of a held handle.
- An `Ok` made of a held handle, `try_borrow_mut()`'s, is recorded too
  (`Made::held_ok`), and its payload read is that handle.
- A guard borrowed `&mut` only to be reborrowed into `deref_mut`,
  `kept(&mut state)`'s, is written once, and so is the `let` that holds
  it, recorded where it's assigned (`derefs_guard`, `Locals::derefed`).

`mem::swap` and `replace` through a held handle assign its place only
where its storage writes the value whole (`Storage::assigns_its_place`).
A flattened state's handle keeps its keys set in place. Tidy's two
repairs are gone.

Without the last three of those, the repairs' removal was a wrong answer
that `main` got right. `if let Ok(mut state) = task.state.try_borrow_mut()
{ halt(Cell::from_mut(&mut *state)); }` assigned `task = value`, and so
did `*kept(&mut state) = ..`, because the handle was reached only through
the `Ok`'s payload and the guard's binding.

One output moved, and it's case A of
[0262](0262-when-rust-and-js-disagree.md) (no observable difference).
`flattened_state_got_mut`'s lent `get_mut()` no longer writes its
copy-back on unwind. Its place is now seen as `task`, a variable of the
body's own that nothing reads once it has unwound, the rule a nullable
place's handle already followed. With a `Drop` holder that reads the
state, the copy-back on unwind is kept and agrees with native Rust.

Held by:
- The corpus cases `stored_place_made_a_mut_dyn`, `flattened_state_handed_on`,
  `flattened_state_lent_through_what_holds_it` (new) and
  `flattened_state_chosen_in_branches` (new), each against native.
- The mutations `flattened-state-lent-unheld`, `held-handle-unnoted`,
  `guard-handle-unnoted`, `flattened-state-assigns-its-place` and
  `swap-into-a-nullable-place-as-given` (retargeted), with the others the
  change lists.
- `pair-of-a-handle-stores-as-given` is retired with the repair it mutated.
- Three checks are kept without a mutation, as nothing found can tell them
  apart. Flush's `const` of a held handle is recorded, but no case lends
  one: the whole corpus and a probe of a nullable `RefCell` and `Cell`
  field lent five ways ran the same without it. A handle stored as it's
  given isn't recorded; recording it gives the same JS in every case
  tried, since a pair on its place and one on its `value` read and write
  alike. A guard's `&mut` counts as reborrowed into `deref_mut` only where
  nothing else uses it (`used_once`); only a write of a whole guard through
  that `&mut` could use it again, which no program found writes.
- `flattened_state_chosen_in_branches` (new) holds the "written once" rule:
  a variable written in two branches records neither handle.

## Amendment: a guard's `&mut` through any name (2026-11-24, #2443)

The amendment above counted a guard's `&mut` as no write of the guard only
where it was reborrowed straight into `deref_mut`, `kept(&mut state)`'s,
and nothing else used it (`used_once`). A `&mut` given a name first, `let
r = &mut state; *kept(r) = ..`, wasn't counted, as borrowck's `FakeRead`
of the `let` was a second use. Its binding wasn't recorded as the
flattened state's handle, and the handle lent assigned the variable
holding the class's object: `undefined Pending { tries: 3 }` where Rust
prints `task Pending { tries: 3 }`. A regression of #2429.

**A guard's `&mut` is no write of the guard where no local that may hold
it can write the guard** (`only_derefed`). The locals that may hold it are
the `&mut`'s own and each written of one of them, moved, copied or
reborrowed (`&mut *r`), through any chain. Each may otherwise be:

- given to `deref_mut` as its one argument;
- borrowed shared, itself or through it, `&r` or `tries(r)`'s `&**r`,
  which only reads (the metamorphic net's `emi-insert` found this one,
  `let _emi = &r;`);
- written;
- read by borrowck alone (`FakeRead`, `PlaceMention`).

Any other use counts the `&mut` as a write of the guard, as before.
`used_once` is gone: the rule asks every use of every holder, which is
what it stood for (#2433's third check).

The rule is narrower than the one under it. A guard given to a function
as a `&mut RefMut`, `touch(r)`, is counted a write, and stays correct
through a handle on its `value`. Counting any use but one that replaces the
guard whole would read better, and is correct as long as a whole
replacement through a `&mut` is refused. That is #2444, which must ask the
owner of those refusals.

No snapshot moved. Held by the corpus case
`flattened_state_lent_through_a_name` (new) against native, and by the
mutations `guard-name-borrowck-read-a-use`, `guard-name-reborrow-unfollowed`,
`guard-name-shared-borrow-a-write` and `guard-name-write-a-use`, each a
wrong answer, and `guard-name-moved-unfollowed`, held by its snapshot: a
holder not followed falls back to a handle on its `value`, right but
wrapped twice.

## Amendment: the held-handle record's last two untested checks are gone (2026-11-24, #2433)

Of the three checks the #2429 amendment kept without a mutation, #2443's
amendment replaced the third (`used_once`). The other two are removed:

- **Flush's `const` of a held handle isn't recorded.** `note_held_handle`
  records a handle where `mir_store` declares its variable, and no longer
  where `flush` makes a `const` of a pending one. No case lends a flushed
  one's `value`. Where one did, the handle lent would be one on its
  `value`, a second wrap that reads and writes through the first, whose
  setter stores as its place does.
- **A handle stored as it's given is recorded too.** `Handled::of` records
  any handle, whatever its storage. A pair or a handle on its place reads
  and writes as one on its `value` does, so the one form serves both.

Proof: with both removed, check:fast over 421 test files and 2,954
corpus cases gave the same JS, every snapshot byte for byte. The probes
#2433 names, and a nullable `RefCell` and `Cell` field lent five ways,
agree with native Rust. Case ([0262](0262-when-rust-and-js-disagree.md)):
none, no output moved.

## Amendment: a handle nothing reads is never read (2026-11-24, #2467)

The #2433 amendment's second removal exposed a rule tidy kept for one
storage only. `Handled::of` now records a handle stored as it's given.
So `std::mem::replace(&mut *holder.slot.borrow_mut(), next)` is lowered
through the record's place, `holder.slot`, and leaves its handle `_11`
read by nothing. Tidy's `read_handles` took a handle no later statement
mentions to be "read at once" (`read_at_once`'s answer with nothing to
look at). It made the dead handle a `const` of its place,
`const slot = holder.slot;`: a read of a JS-made field's getter that the
program never made, two reads where Rust's one. `getter-reads.test.ts`'s
`getter_closures` found it; the corpus can't see a getter's reads.

`read_handles` already said so of a handle on a nullable place (#541's
"nothing reads: none"). That was one instance of the general rule: making
a handle reads nothing, so a handle nothing reads is never read at once,
whatever its storage. The #2433 removal stands: nothing tells the
`AsGiven` check apart again.

Case ([0262](0262-when-rust-and-js-disagree.md)): the third, a getter's
reads, which Rust's are. No corpus snapshot moved over its 2,966 cases.
Held by `getter_closures` and the mutation `dead-handle-read-at-once`.
`readability-passes.yml`'s paths now include the handles' makers and
their record (`mut_refs`, `noted`, `js/writes.rs`, MIR's
`temporaries.rs`), so a change to them runs the read cases too.
