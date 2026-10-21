# 0534. A bound of a sequence's `Index<J>` is its `J: SliceIndex<[T]>`

Status: Accepted. Extends [0418](0418-a-generic-slice-index-is-a-dictionary.md).

Case: D ([0262](0262-when-rust-and-js-disagree.md)) closed: what was
rust-js's own error compiles as Rust's does.

## Context

Generic code indexes a `Vec` or a slice by an index it's given two ways: a
bound on the index, `J: SliceIndex<[T]>`, whose dictionary indexes (ADR
0418), or a bound on the sequence, `Vec<T>: Index<J>`, as itertools'
`LazyBuffer` writes it:

```rust
impl<I, J> Index<J> for LazyBuffer<I>
where
    Vec<I::Item>: Index<J>,
{
    type Output = <Vec<I::Item> as Index<J>>::Output;

    fn index(&self, index: J) -> &Self::Output {
        self.buffer.index(index)
    }
}
```

`Index` has no dictionary, so the second was given nothing, and its
`index` was refused: "implementation evidence for `<J as
SliceIndex<[T]>>`".

## Decision

**A bound of `Index<J>` or `IndexMut<J>` on a `Vec<T>`, an array or a
slice, of an index generic code is given, is the bound it holds by,
`J: SliceIndex<[T]>`: std's one impl of them asks it.** Its evidence is
that dictionary, given and passed as ADR 0418's is.

```js
function bufferIndexJ_index(buffer, index, JSliceIndexTSlice) {
  return JSliceIndexTSlice.index(index, buffer.buffer);
}
```

- `recognition::index_by_slice_index` answers which bound it is, and
  `bound_of`, where a clause becomes evidence, the callee's and its
  callers' alike, asks it. A bound of both is one dictionary.
- A bound of a known index, `Vec<T>: Index<usize>`, indexes in place, as
  it did: it's given nothing.
- **`index_mut` gives generic code's `&mut`, a box; where what it gives
  is known here**, a bound's `IndexMut<J, Output = [u32]>`, it's what's in
  the box, as a generic function's result is (ADR 0099). A cell's,
  `String`'s, stays its handle.

## Why

`index_bound_generic` indexes a struct by an item, ranges of each kind
and all of it through its `Vec`'s `Index` and `IndexMut`; a slice by an
item and a range; an array's `String` in place; a `Vec`'s range reversed
in place through a bound of both, and by a known index; each against
native Rust.

## Consequences

- itertools' `LazyBuffer` compiles.
- Mutations drop `IndexMut`, arrays or `Vec`s, give a known index a
  dictionary, keep the bound as it is, leave a known `index_mut` in its box,
  and take a `String`'s out of its handle.
