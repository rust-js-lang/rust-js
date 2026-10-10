# 0337. A slice as arrays of `N`, and arrays as one slice

Status: Accepted. Extends [0335](0335-slice-views.md) and
[0324](0324-slice-methods.md); counted by
[0314](0314-std-data-structures.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A slice's `as_array`, `array_windows`, `as_chunks` and `as_rchunks`, which
see it as arrays of a constant `N`, and `as_flattened`, which sees arrays
as one slice, were errors, and so were their `_mut` ones. A `&mut` of part
of a slice is a view (ADR 0335), but `as_chunks_mut`'s `&mut [[T; N]]` is
a slice whose items are arrays, each of which may be written whole:
`chunks[1] = [9, 9, 9]`, `chunks.swap(0, 1)`.

## Decision

**The shared ones are copies, as a shared slice is (ADR 0063); the `_mut`
ones are views. A view of chunks is a view whose items are views of
them, and whose item written whole writes its items.**

```rust
let (chunks, rest) = v.as_chunks_mut::<3>();
chunks[0][2] = 0;
chunks.swap(0, 1);
let flat = grid.as_flattened_mut();
flat.reverse();
```

```js
const [chunks, rest] = $asChunksMut(v, 3);
$index(chunks, 0)[2] = 0;
$swap(chunks, 0, 1);
const flat = $flatView(grid, 2);
flat.reverse();
```

| Rust | JS |
|---|---|
| `as_array::<N>()`, `as_mut_array::<N>()` | `v.length === N ? v : undefined`: itself, as long |
| `array_windows::<N>()` | `$windows(v, N)`, as `windows(N)` |
| `as_chunks::<N>()`, `as_rchunks::<N>()` | `$asChunks(v, N)`, `$asChunks(v, N, true)`: copies of its whole chunks and what's left, after them or before |
| `as_chunks_mut`, `as_rchunks_mut` | `$asChunksMut(v, N)`: a view of chunks, and a view of what's left |
| `as_chunks_unchecked(_mut)` | the chunks of `$asChunks(v, N)`, or `$asChunksMut`'s |
| `as_flattened()`, `as_flattened_mut()` | `v.flat()`, `$flatView(v, N)`: a view of each array's items, in order |

- **A chunk read before its place is written keeps what it held**, as
  Rust's copy of a `[T; N]` does. JS's own `reverse` and `sort`, and
  `$swap`, hold one item while they write its place: a view that followed
  its place would have `chunks.swap(0, 1)` copy one chunk into both.
  `let kept = chunks[0]` is a copy, `.slice()`, where it's changed after.
- **`$view` is a `$proxy(length, read, write)`**, and so are the view of
  chunks and the flattened view: one spell for a view, three reads and
  writes.

## Why

- **It's the same program**: each read and write is of the item Rust's
  is, a chunk written whole writes its items, and one moved by `swap`,
  `reverse` or `sort` moves its items, as Rust's do.
- **It's the JS a person writes**: a shared one is arrays, a `_mut` one
  is used as an array is.
- **It's tested**: the `slice_arrays` corpus case runs, against native
  Rust, `as_array` of a slice as long and not, `array_windows` taken apart
  as arrays, `as_chunks`, `as_rchunks`, `as_chunks_unchecked` and
  `as_flattened`; `as_mut_array`, and `as_chunks_mut` written through a
  chunk's item, a whole chunk, `swap`, a copy kept while its chunk
  changes, `as_rchunks_mut` reversed and each chunk reversed, and
  `as_flattened_mut` written, filled through a view and reversed.
  Mutations let a read chunk follow its place, swap what's left and the
  chunks, start `as_rchunks` at the start, read a flattened view across,
  take any length for `as_array`, window by one, copy for each `_mut`, and
  give `as_chunks_unchecked` the pair.
- `docs/std-coverage.txt`: `slice` 107 of 133.
