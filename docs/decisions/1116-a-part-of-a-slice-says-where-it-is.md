# 1116. A part of a slice says where it is

Status: Accepted (the user's decisions, 2026-11-10). Amends
[0063](0063-text.md) and [0335](0335-slice-views.md): a part of a slice
is still a copy, or a view of a `&mut` one, and now says where it is in
the array it's of; extends [0562](0562-a-pointer-is-a-place-and-an-address.md)'s records (#1113,
#1114) to parts.

Case ([0262](0262-when-rust-and-js-disagree.md)): R. Results changed: a
part's address was another array's, a wrong answer, so Rust's is kept.

## Context

`&v[1..3]` is a copy of those items in JS, `$slice(v, 1, 3)`, as nothing
changes what a shared part is of while it's borrowed (ADR 0063); a
`&mut` one is a view, `$view(v, 1, 3)` (ADR 0335). Each is an array of its
own to JS, so its address was its own: `a.as_ptr() == b.as_ptr()` and
`ptr::eq(a, b)` of two `&v[1..3]` were `false`, where Rust's are `true`,
and `ptr::eq` of slices compared which arrays JS held them as, not where
they start and how long they are (#1185, found beside #1183). The refusal
of a pointer into a copy (`Pointed::copies`) saw only MIR's subslice
patterns and named constants.

The options were a record of where each part is, a refusal of a part's
address, or both by crate. Records only in crates that take an address
were the first choice; but code that runs as its module loads, a
thread-local's initializer (ADR 1006) or a `js::on_load!`, runs before
the crates that import it, so a library's part made then would be
unrecorded, a wrong answer no run could tell from a right one. A record in
a `WeakMap` cost ten times a part's making (607–645 ms against 48–54 ms
for `&v[i..i + 4]` of each `i` of 1e6 items); a property under a symbol
costs 1.4–1.9 times (67–102 ms), on a quiet machine (load 1.82).

## Decision

**Every part of a slice records where it is: `part[$placeOfPart] = { of,
start }`, of the array that holds its items itself, its offset there, in
every crate, by the runtime.** `$slice`, `$sliceGet`, `$sliceSplitAt`,
`$windows`, `$chunks`, `$chunksExact`, `$rchunks`, the splits by
predicate, `chunk_by`, `split_off`, `as_chunks`, `strip_prefix` and
`trim_ascii` cut parts with `$part(items, start, end)`; a `&mut` view's
record is on its `Proxy`'s target, which its `get` reads. A pointer into a
part, `$ptr(part, i)`, is one into that array, `$ptr(of, start + i)`, so
its address, its order and what it reads are Rust's. A part of a part is
recorded in the outermost array.

| Rust | JS |
|---|---|
| `&v[1..3]`, checked | `$slice(v, 1, 3)`, a part, recorded |
| `&v[1..3]` in bounds, of a crate that takes no slice's address | `v.slice(1, 3)`, as before |
| `&v[1..3]` in bounds, of a crate that takes one | `$part(v, 1, 3)`, recorded |
| `v[a..b].to_vec()`, `into()`, a `collect()` of its items copied | `$copyOf(v, a, b)`: one copy, owned, no record, `$slice`'s panics |
| `ptr::eq(a, b)` of two slices, `==` of two `*const [T]` | `$slicePtrEq(a, b, size)`: where each starts and its length |

- **A crate takes a slice's address** (`StoredPlaces::takes_slice_addresses`,
  beside `asks_addresses`, the one owner): a raw pointer to a slice or an
  array, `ptr::eq`'s of two slices; a slice's, an array's or a `Vec`'s
  `as_ptr()`, `as_mut_ptr()` or their ranges; a pointer to an item of
  one; or any address asked. Only such a crate's in-bounds parts, which
  JS's own `slice` makes inline, say where they are (`slice_part`, the
  one place a part is made inline): a crate that takes none writes what
  it wrote, byte for byte, which `test/lowering-collections.test.ts`
  holds.
- **An owned copy is an allocation of its own.** A part that says where
  it is can't be the `Vec` that owns those items, or a pointer into the
  `Vec` would read what `v` holds: `$slice` is off the list of arrays made
  here alone (`iterators::eager::fresh`), and a range's items taken as
  owned are `$copyOf(v, a, b)` (`iterators::owned_items`), the one owned
  form, checked as `&v[a..b]` is, one copy. Ten corpus snapshots read
  `$copyOf` where they read `$slice`.
- **Text keeps its refusal.** A `&str`'s part is a JS string, a value with
  no place, as its bytes are (ADR 0126, amended): `ptr::eq` of them is
  refused, as before.

The record is under a symbol, which JS's own reads of an array, its items,
its length, `for .. of`, `Object.keys` and `JSON`, never see;
`Object.getOwnPropertySymbols` and an object spread `{ ...part }` would.

## Consequences

- Of `&v[i..i + 4]` made for each `i` of 1e6 items, 1.4–1.9 times the
  time (48–54 ms to 67–102 ms); of using a part, none: a hot sum of 1e6
  items 20 times, untagged 7.6–7.7 ms, tagged 7.5–7.6, and a loop given
  both by turns 7.5–7.6; index writes, untagged 5.6–5.7 ms, tagged 5.7,
  by turns 6.7, a case no program reaches, as a tagged array is never
  written: a shared part is read only, a `&mut` one is a view, and an
  owned copy has no record (quiet M6, load 1.82, 2026-11-10).
- The corpus case `slice_parts_by_address.rs`: `as_ptr()`, `ptr::eq` and
  `==` of parts of each kind, a part of a part, an empty one at its place,
  a view written through `as_mut_ptr()`, a pattern's rest, `first_chunk`
  and `last_chunk`, and owned copies that aren't `v`'s, against native
  Rust in both builds; #1185's own program prints `true true`.
- Still a copy's address: a MIR subslice pattern's rest of a `&` and a
  named constant, `Pointed::copies`, whose pointers stay refused.

## Amendment: a pointer to a `Vec` is to the vector, not into its items (2026-11-10)

`takes_slice_addresses` took any raw pointer to a slice, an array or a
`Vec` for a slice's address, so a crate with a `thread_local!` of a `Vec`,
whose initializer std reads through a `*const Vec<T>`, wrote `$part(..)`
for every in-bounds part, and React's port's `react-dom-bindings` lost
upstream's `list.slice(0, list.length - 1)`. **A raw pointer to a `Vec`,
that one or `ptr::eq` of two `Vec`s, is to the vector, which is the array
in JS whatever its parts are: it takes no slice's address.** Only a raw
pointer to a slice or an array is one; a `Vec`'s items are still reached
by its `as_ptr()` family or a pointer to an item, `&v[1] as *const T`,
which are. `test/lowering-collections.test.ts` holds a crate with both
writing `v.slice(1, 3)`, and the mutation `slice-addresses-of-a-vec-pointer`
puts the `Vec` back.
