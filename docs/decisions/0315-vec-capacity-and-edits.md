# 0315. A `Vec`'s capacity does nothing, and its edits are std's

Status: Accepted. Extends [0025](0025-vec-loops-refcell-mut.md) and
[0153](0153-collection-and-cell-methods.md); counted by
[0314](0314-std-data-structures.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)), but for `capacity()`,
D: refused.

## Context

ADR 0314's count had `Vec` at 18 of its 48 methods: even
`Vec::with_capacity`, `swap_remove` and `resize` were refused.

## Decision

**A JS array has no capacity, so asking for some does nothing, what's
given still run; and `Vec`'s edits are std's, by a helper each where JS
has none.**

| Rust | JS |
|---|---|
| `Vec::with_capacity(n)` | `[]` |
| `v.reserve(n)`, `reserve_exact`, `shrink_to`, `shrink_to_fit` | nothing (`n` still runs) |
| `v.swap_remove(i)` | `$swapRemove(v, i)`, which panics as std does |
| `v.resize(n, x)` | `$resize(v, n, x)`, or `$resize(v, n, x, clone)` of what copies can be told apart: a clone each but the last, `x` itself |
| `v.resize_with(n, f)` | `$resizeWith(v, n, f)` |
| `v.dedup_by(f)`, `dedup_by_key(k)` | `$dedupBy(v, f)`, `(a, b) => k(a) === k(b)` |
| `v.pop_if(f)` | `$popIf(v, f)` |
| `v.extend_from_within(a..b)` | `v.push(...$slice(v, a, b))` |
| `v.splice(a..b, items)` | `$splice(v, a, b, items)`, checked as `drain` is |
| `v.into_boxed_slice()`, `into_flattened()` | `v`, `v.flat()` |

- A closure given `&mut` items, `dedup_by`'s or `pop_if`'s, is given an
  object itself, or a cell of a number or text, whose change is written
  back, as std's are: `$dedupByCells`, `$popIfCell`.
- `VecDeque`'s and `BinaryHeap`'s `reserve` and the rest are the same.
- `capacity()` and `try_reserve` stay refused: JS keeps no capacity to
  say, and a number made up would break what std promises of it.

## Why

- **It's the same program**: capacity is never seen but by `capacity()`,
  which is refused; each edit is std's, by its steps.
- **It's tested**: a corpus case runs each against native Rust, with a
  capacity's argument that does something, copies told apart, and
  closures that change what they're given; mutations refuse
  `with_capacity`, drop the argument, share copies, give text where cells
  are, and break each helper.
- `docs/std-coverage.txt`: `Vec` 33 of 48, `VecDeque` 34 of 55.
