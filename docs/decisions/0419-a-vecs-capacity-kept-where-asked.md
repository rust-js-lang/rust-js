# 0419. A `Vec`'s capacity is kept where it's asked

Status: Accepted. Amends [0315](0315-vec-capacity-and-edits.md), which
refused `capacity()`. Amended by
[0774](0774-room-past-isize-max-bytes-panics.md): the room asked is checked
first.

Case: C ([0262](0262-when-rust-and-js-disagree.md)) for what std promises
of a capacity; B for its number, which native Rust's growth decides and
std doesn't promise, listed below.

## Context

JS's array has no capacity, and ADR 0315 refused `capacity()`: a number
made up would break what std promises of it, `Vec::with_capacity(10)`'s
`capacity() >= 10`, and that `clear()` leaves it. smallvec asks it, in
`from_vec`, to keep a `Vec`'s room as its own: refused, smallvec was.

## Decision

**A `Vec`'s capacity is kept, `$capacities`, where the crate asks it, or a
library it uses does: at least its length, what `with_capacity` and
`reserve` gave, and never less where it's shortened.**

| Rust | JS, where kept | JS, where not |
|---|---|---|
| `Vec::with_capacity(n)` | `$withCapacity(n)` | `[]`, as before |
| `v.capacity()` | `$capacity(v)`: its kept room, or its length, the more | (it's kept, as it's asked) |
| `v.reserve(n)`, `reserve_exact(n)` | `$reserve(v, n)` | nothing |
| `v.shrink_to(n)`, `shrink_to_fit()` | `$shrinkTo(v, n)`, of `0` | nothing |
| `clear()`, `truncate`, `set_len`, `pop`, `drain`, … | `$keepCapacity(v)` first | as before |

- **Which `Vec` methods ask or change its capacity is
  `recognition::capacity_op`'s**: `with_capacity`, `reserve` and the rest
  are its alone, where it isn't kept too, which ADR 0315's arms of them
  answered a second time, unreached, and are gone.
- **Which `Vec` methods shorten it is `recognition::shortens_vec`'s**,
  asked once, before any std call is lowered: what it has kept first.
- **Where it's kept is the crate's, `asks_capacity`, or a library's**: a
  library's manifest says it keeps capacity, `keeps_capacity`, and its
  consumers keep theirs, as a vector they give it may be asked. A crate
  that neither asks nor uses one that does is the JS it was.

## Why

- **It keeps std's promises**: `test/corpus/vec_capacity.rs` asks a
  `with_capacity`'s, a `reserve`'s, and one left by `clear()`, `truncate`
  and `pop`, as native Rust answers; `test/crates.test.ts` a library's of a
  consumer's vectors.
- **It costs only who asks**: a crate that never asks a capacity, and
  uses no library that does, gains nothing.

## Consequences

- **Its number isn't native Rust's**: a `Vec` grown by `push` has its
  length as its capacity, where native Rust's has doubled. std promises a
  capacity is at least what's needed, not which it is: a program that
  prints one sees a different number (case B).
- `VecDeque`'s, `String`'s and a map's capacity stay refused.

## Amendment: a `VecDeque`'s and a `BinaryHeap`'s

A `VecDeque` and a `BinaryHeap` are JS arrays too (ADR 0068), and their
capacity is their array's. Which collections are so is
`recognition::ARRAY_COLLECTIONS`, the one list: `with_capacity`,
`capacity()`, `reserve`, `reserve_exact`, `shrink_to` and `shrink_to_fit`
of each are `capacity_op`'s, and what shortens each, `pop_front`,
`pop_back`, a heap's `pop`, `clear`, `drain`, `retain` and the rest, and
`append`'s other, `shortens_vec`'s, keeping what it had. Their
`with_capacity` was their `new`, a second answer to what it is, and is
`capacity_op`'s alone. The `collection_capacity` corpus case asks a heap
made with room, reserved, popped and cleared, one grown by pushes and
cleared, and a deque so, with native Rust; rustc's
`zero-sized-binary-heap-push` passes.
