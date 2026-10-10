# 0335. A `&mut` of part of a slice is a view of those items

Status: Accepted. Amends [0063](0063-text.md); extends
[0324](0324-slice-methods.md) and [0152](0152-std-item-handles.md);
counted by [0314](0314-std-data-structures.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A slice of a slice by a range is a copy, which a shared one can be:
nothing changes what it's of while it's borrowed (ADR 0063). A `&mut` one
can't be a copy, as what's written through it must reach the slice, so
`&mut v[a..b]`, `v[1..].sort()`, `split_at_mut`, `chunks_mut` and every
other `_mut` split were errors. ADR 0063 put off a view because each read
would go through it. `slice` was 67 of 133.

## Decision

**A `&mut [T]` of part of a slice is a view: a JS `Proxy` of an array
whose items are those of the slice, `$view(v, a, b)`. JS's own array
methods, `sort`, `reverse`, `fill`, a `for .. of` and the rest, read and
write it as they do an array.**

```rust
v[1..4].sort();
let (left, right) = v.split_at_mut(3);
left[0] = right[0];
for chunk in v.chunks_mut(4) {
    chunk[0] *= 2;
}
```

```js
$view(v, 1, 4).sort((a, b) => a - b);
const [left, right] = $splitAtMut(v, 3);
left[$at(left, 0)] = $index(right, 0);
for (const chunk of $chunksMut(v, 4)) {
  chunk[$at(chunk, 0)] = Math.imul(chunk[$at(chunk, 0)], 2);
}
```

| Rust | JS |
|---|---|
| `&mut v[a..b]`, `get_unchecked_mut(a..b)` | `$view(v, a, b)`, checked as `&v[a..b]` is, as it's made; `&mut v[..]` is `v` |
| `v.get_mut(a..b)` | `$viewGet(v, a, b)`: a view, or `undefined` |
| `split_at_mut(mid)`, `_checked`, `_unchecked` | `$splitAtMut(v, mid)`: two views, `mid > len` past the end, or `undefined` checked |
| `split_first_mut()`, `split_last_mut()` | `$splitEndMut(v, last)`: the item, a handle on a number or text, and a view of the rest |
| `chunks_mut`, `chunks_exact_mut`, `rchunks_mut`, `rchunks_exact_mut` | `$chunksMut`, `$chunksExactMut`, `$rchunksMut`: views, `into_remainder()` the last |
| `split_mut`, `splitn_mut`, `rsplit_mut`, `rsplitn_mut`, `split_inclusive_mut` | `$sliceSplitByMut`: views |
| `chunk_by_mut(p)`, `chunk_by(p)` | `$chunkByMut`, `$chunkBy`: the runs `p` holds of each two in a row of |
| `first_chunk_mut::<N>()`, `last_chunk_mut`, `split_first_chunk(_mut)`, `split_last_chunk(_mut)` | a view of those `N`, or copies of the shared ones |
| `split_at_unchecked(mid)`, `get_unchecked_mut(i)` | `split_at`'s copies, `$mutAt(v, i)` |

- **One spell for a view**: each `_mut` helper is its shared one's, cut as
  views, not copies: `$chunksMut(v, n)` is `$chunks(v, n, $view)`.
- **A view stays over the same items**: nothing a `&mut [T]` does makes it
  longer or shorter, and its slice can't be changed but through it while
  it's borrowed. Its `length` is set only as `$assign` sets it, to what it
  is, so `*chunk = [7, 8]` writes its items (ADR 0147).
- **A view of a view** is a view of those items of the first.
- **A shared slice stays a copy** (ADR 0063): a plain array, which reads
  fastest, and the JS a person writes.
- **`remainder()` of a `ChunksExact` stepped through a `&mut`** is read off
  what it steps, `exact.items.remainder` (ADR 0071): a shared one's was
  lost there too.
- **Still refused**: `as_chunks_mut` and `as_rchunks_mut`, whose `[[T; N]]`
  of chunks would need a view whose items are views written whole;
  `get_disjoint_mut`, whose error is a std enum rust-js has no form for
  yet; `split_off*`, which change the `&mut &mut [T]` they're given;
  `select_nth_unstable*`, whose order is std's algorithm's; and what needs
  where a slice is in memory, `as_ptr`, `align_to`, `subslice_range`.

## Why

- **It's the same program**: every write through a view is the slice's,
  as Rust's is, and every view is checked, and panics, where Rust's is
  made.
- **It's the JS a person writes, where it can be**: `$view(v, 1, 4).sort()`
  is the call it is in Rust, and a JS array's methods are used on it as
  they are; only a `&mut` of part of a slice is a view.
- **It costs what it must**: a `Proxy`'s reads are slower than an array's,
  and only a `&mut` of part of a slice pays it. Sorting 100,000 items
  through one takes 30 ms in Bun, 18 in Node.
- **It's tested**: the `slice_views` corpus case runs, against native
  Rust, `sort`, `reverse`, `fill`, `swap` and `rotate_left` of ranges, a
  view kept, given and returned, a view of a view, `&mut v[..]`,
  `split_at_mut` and its checked and unchecked forms, `split_first_mut`
  and `split_last_mut` of numbers, each `chunks*_mut` with `into_remainder`
  of one stepped through a `&mut`, each `split*_mut`, `get_mut`,
  `get_unchecked_mut`, the `*chunk_mut`s with a whole chunk written, and
  `chunk_by(_mut)`, and splits of structs. `slice_view_past_len` panics as
  a view is made, and `split_at_mut_past_len` with `mid > len`. Mutations
  leave a view unchecked, unwritten, unshifted or as long as its slice,
  refuse its length, cut copies for each `_mut` helper, drop a handle, the
  checked forms and the stepped `remainder`, and cut each split, run and
  chunk wrong.
- `docs/std-coverage.txt`: `slice` 91 of 133.
