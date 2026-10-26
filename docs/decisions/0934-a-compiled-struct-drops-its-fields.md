# 0934. std's struct compiled for a build drops what its fields hold, as a struct of the crate's does

Status: Accepted, under the user's goal of 2026-10-26 ("keep going until
all done, the choice is your recommendation"). The plan "rust-js: one
representation per iterator", its second phase. Extends
[0485](0485-an-adapter-holding-a-function-is-closed-to-drop-it.md) and
[0561](0561-std-bodies.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): Rust's behavior is
kept where rust-js refused it. With std's structs alone, a `scan()` or a
`flat_map()` of items with a destructor was refused, "a std type holding
a value with a destructor".

## Context

What drops a std adapter is a rule of each kind: one holding a function
closes its source and drops its function (ADR 0485), one closed to drop
what it holds names it (`closed_on_drop`), and any other holding what has
a destructor is refused. But std's struct compiled for the build is a
struct as the crate's are, whose maker gives it a `close` that drops each
field (`iterator_map`'s `close(iterator) { dropSelf?.(iterator.iter);
dropF?.(iterator.f); }`): a `Scan`'s, which no rule named, and the private
struct a `Flatten` holds, `FlattenCompat`, which isn't an iterator of
`std::iter` at all.

## Decision

- **std's struct compiled for this build, or a private struct with bodies
  `@rust-js/std` compiles, drops what its fields drop** (`drops_in`, of
  `has_compiled_bodies`): its own `close` drops them, in their order, as
  Rust's drop glue does.

## Consequences

- ADR 0908's `differs` is 10, from 13: `scan_lazy_drops.rs`,
  `held_iterator_dropped_by_its_owner.rs` and `flat_map_drops.rs` run as
  native Rust does with std's structs alone; the first and last join the
  suite's few.
- The normal build's JS doesn't change. Its held `scan()` over what has a
  destructor is still refused there, `Scan` being `MEANING_ONLY` (ADR
  0932) till it's held.
- `result_given_as_items.rs` compiles with std's structs alone now, and
  throws: std's `Chain` given a `&mut` to a `Result` as its `U:
  IntoIterator` gets it as it is, but the dictionary made for it takes the
  box generic code has of a `&mut C` (ADR 0457). Open.
- Mutations: `compiled-struct-drops-refused`, `compiled-bodies-part-drops-refused`.
