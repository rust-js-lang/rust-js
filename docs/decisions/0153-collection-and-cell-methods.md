# 0153. Collection and cell methods, and `to_vec()` cloning what it copies

Status: Accepted. (Amended: a slice's `split_at`, `get` of a range,
`starts_with` and `ends_with`, and `eq_ignore_ascii_case` of bytes, which
stopped uuid and writeable; `split_first` and `split_last`, chrono's.) Extends [0036](0036-iterators-and-sorting.md),
[0052](0052-std-trait-impls.md), [0062](0062-combinators.md) and
[0025](0025-vec-loops-refcell-mut.md).

Case: C, D ([0262](0262-when-rust-and-js-disagree.md)).

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
| `s.split_at(mid)` | `$sliceSplitAt(s, mid)`, two copies, as `&s[..mid]` is; past the end it panics, `mid > len` |
| `s.split_at_checked(mid)` | `$sliceSplitAt(s, mid, true)`, `undefined` past the end |
| `s.split_first()`, `split_last()` | `s.length === 0 ? undefined : [s[0], s.slice(1)]`, the rest a copy |
| `s.get(a..b)` | `$sliceGet(s, a, b)`: a copy, or `undefined` where `&s[a..b]` would panic |
| `s.starts_with(p)`, `s.ends_with(p)` of numbers, strings or `bool`s | `$sliceStartsWith(s, p)`, `$sliceStartsWith(s, p, true)`, each item by `===` |
| `a.eq_ignore_ascii_case(b)` of bytes | `$bytesAsciiEq(a, b)` |

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

## Amendment: `Cell::from_mut`, and a cell of each item

`Cell::from_mut(r)` sees a `&mut` as a `&Cell`, `get_mut`'s inverse: of a
number or text, the cell the `&mut` already is, `{ value }`; of an object,
an array or a struct, `$cellOf(object)`, a cell whose `value` is the object
and whose write changes it in place, as writing through the `&mut` does,
`$assign`. A `&Cell<[T]>`'s `as_slice_of_cells()`, and a `&Cell<[T; N]>`'s
`as_array_of_cells()`, are a handle on each item, `$mutItems`, of what the
cell holds, read straight through a `$cellOf`. The `cell_from_mut` corpus
case writes a number, a slice's items, an array's, a whole array through a
parameter and strings, against native Rust; rustc's
`rfcs/rfc-1789-as-cell/from-mut.rs` passes.

## Amendment: a slice's `to_owned()`, and no copy of an array made here

**A slice's `to_owned()` is its `to_vec()`**, as `str`'s is its `String`:
percent-encoding's `bytes[..n].to_owned()`, which was refused. And
`to_vec()` of an array made where it's called, which nothing else holds,
`$slice(bytes, 0, n)` or `words.map(f)`, is that array, not a copy of it,
where its items need no clone; whether it's made here is
`iterators::made_array`'s answer, which collecting asks too. A literal's
items are made with it, so it's itself whatever they are.

## Amendment: `binary_search` and `dedup_by_key` of any item

`binary_search` of items `<` doesn't order as `Ord` does, a derived enum's
or a struct's, was refused (#151), and strings went their own way.
`binary_search` is `$binarySearch(s, x)` where `<` orders them, and
`$binarySearchBy(s, (each) => ..)` of their `Ord`, as `sort()` orders them,
for any other: strings, `char`s, a derived or a hand-written `Ord`.
`dedup_by_key` of keys `===` doesn't compare, an `Option`, a tuple, was
refused (ADR 0315's `k(a) === k(b)`); its keys are compared by their `==`,
each named first where that reads it more than once: `const aKey = k(a)`.
Held by `test/corpus/search_by_ord.rs` and the mutations
`binary-search-by-lt` and `dedup-key-by-identity`.

## Amendment: a `T: Clone`'s `to_owned()` is its `clone()` (#316)

`to_owned()` was a `str`'s `String` and a slice's `to_vec()`; of anything
else, std's blanket `impl<T: Clone> ToOwned for T`, it was refused:
TheAlgorithms/rust's activation functions, `let mut _vector =
vector.to_owned();` of a `&Vec<f64>`. That impl's body is `self.clone()`,
so **a `to_owned()` std doesn't know otherwise is `Clone::clone` of the
same `Self`**, `vector.slice()`, a struct's copy, `TClone.clone(x)` of a
generic `T`; a `Path`'s and an `OsStr`'s too, a string as a `PathBuf` and
an `OsString` are. A slice's stays its `to_vec()`: a clone of a `[T]` is
the array it's given, so a change to the copy would change the original
(`to_owned_of`, in recognition). Held by the corpus case
`to_owned_is_clone`, which changes a plain slice's copy too, and the
mutations `blanket-to-owned-shared` and `slice-to-owned-unknown`.

