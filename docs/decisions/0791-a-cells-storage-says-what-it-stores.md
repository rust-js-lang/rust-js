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

