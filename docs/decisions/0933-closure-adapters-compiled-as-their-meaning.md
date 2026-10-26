# 0933. `FilterMap`, `TakeWhile`, `SkipWhile`, `MapWhile` and `Scan` are compiled as their meaning, and a function given to std's compiled maker is given its state's box

Status: Accepted, under the user's goal of 2026-10-26 ("keep going until
all done, the choice is your recommendation"). The plan "rust-js: one
representation per iterator", its second phase. Extends
[0932](0932-an-adapter-compiled-as-its-meaning-alone.md) and
[0454](0454-a-mut-given-as-a-type-parameter.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): results change.
With std's structs alone, `scan((0u64, 1u64), |pair, _| { *pair = .. })`
threw `Cannot convert NaN to a BigInt`, and a `take_while` or a `scan` of
a `skip()` counted asked its source `nth(2)` where Rust asks `nth(1)`.

## Context

Compiled as their meaning (ADR 0932), the closure-holding adapters found
two things:

- std's `Scan::next` gives its function `&mut self.state`, which, of a
  type parameter, generic code holds in a box; the crate's closure, of a
  tuple, takes the tuple itself. A function given to the crate's own
  generic code is wrapped to take the box, `(x0) => f(x0.value)` (ADR
  0454), but not one given to a trait's provided method, `scan`, which had
  no impl to say which of its parameters are boxes.
- `TakeWhile`, `MapWhile` and `Scan` write their `fold` by
  `impl_fold_via_try_fold!`, which a `count()` or a `last()`, by std's
  default `fold`, asks.

## Decision

- **`FilterMap`, `TakeWhile`, `SkipWhile`, `MapWhile` and `Scan` are
  `MEANING_ONLY`**: compiled whole, their generated `fold`s too.
- **A function given to a provided method std's body is compiled of is
  given what that body gives it**, its state's box unboxed for it, as one
  given to the crate's own generic code is.

## Consequences

- The build of std's structs alone checks each of them; the normal build's
  JS doesn't change.
- ADR 0908's `differs` is 13: `held_iterator_dropped_by_its_owner.rs` and
  `scan_lazy_drops.rs`, which that build couldn't check before, are
  refused there for a `Scan` holding a value with a destructor, as
  `flat_map_drops.rs` is for a `Flatten`. `scan_object_state.rs` joins the
  suite's few.
- Mutations: `compiled-std-maker-callee-unseen`.
