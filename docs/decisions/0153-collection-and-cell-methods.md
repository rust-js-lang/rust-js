# 0153. Collection and cell methods, and `to_vec()` cloning what it copies

Status: Accepted. Extends [0036](0036-iterators-and-sorting.md),
[0052](0052-std-trait-impls.md), [0062](0062-combinators.md) and
[0025](0025-vec-loops-refcell-mut.md).

## Context

App code stopped at methods each one line long: `extend_from_slice`,
`binary_search_by`, `binary_search_by_key`, `rotate_left` and
`rotate_right`, `[..].into()` of a set or a map, and `Cell::replace`,
`Cell::take`, `RefCell::replace`, `RefCell::take` and `replace_with`.

Probing them found `to_vec()` wrong: it copied the array, not its items, so
changing a struct in the copy changed the original's.

```rust
let mut copied = origin.to_vec();
copied[0].x = 9; // `origin[0].x` was 9 too
```

## Decision

| Rust | JS |
|---|---|
| `v.extend_from_slice(&s)` | `$extend(v, s)`, each item a clone where its type needs one |
| `s.to_vec()` | `s.slice()`, or `s.map((item) => ({ ...item }))` where items need cloning |
| `s.binary_search_by(f)` | `$binarySearchBy(s, f)` |
| `s.binary_search_by_key(&b, f)` | `$binarySearchBy(s, (item) => $cmp(f(item), b))` |
| `v.rotate_left(n)`, `rotate_right(n)` | `$rotateLeft(v, n, "mid")`, `$rotateRight(v, n, "k")` |
| `[..].into()` of a set, a map, a queue, a heap or a `Vec` | what its `from` is: `new Set([..])` |
| `c.replace(v)`, `c.take()` of a `Cell` or `RefCell` | `$cellReplace(c, v)`, `$cellReplace(c, 0)` |
| `c.replace_with(f)` | `$cellReplace(c, f(<&mut to its value>))` |

- **A binary search calls its comparison as Rust's does:** halving, then on
  the last item left, so a comparison that prints or counts sees the items
  Rust's would. An `Ordering` is -1, 0 or 1 (ADR 0036).
- **A clone where Rust makes one:** `to_vec()` and `extend_from_slice` clone
  each item, which `{ ...item }` is for a struct that's changed somewhere
  (ADR 0052), and nothing for numbers and strings. `VecDeque::from(v)` takes
  `v` whole, as Rust moves it.
- **Rotating past the end fails std's assertion,** which names the count
  as std does: `mid <= self.len()` of a slice's `rotate_left`, `k` of its
  `rotate_right`, and `n` of a `VecDeque`'s.
- **`take()` leaves its type's default,** as `mem::take` does.
- **`replace_with`'s closure is given a `&mut` to the value:** the object,
  or for a number or a string the cell's own `{ value }`, which is a box
  (ADR 0074). Rust calls it before replacing, as `$cellReplace(c, f(c))` does.

## Why

- **It's exact:** the `collection_methods` corpus case compares each with
  native Rust, structs changed after `to_vec()` and `extend_from_slice`
  included, and `rotate_past_end` and `deque_rotate_past_end` the panics.
- **It's the JS a person writes:** `$binarySearchBy(sorted, (p) => $cmp(p,
  30))`, and `const old = $cellReplace(count, 6)`.

## Consequences

- `to_vec()` of structs that are changed somewhere copies each one, as
  `clone()` of the `Vec` does.
- `Cell::swap` is still an error.
