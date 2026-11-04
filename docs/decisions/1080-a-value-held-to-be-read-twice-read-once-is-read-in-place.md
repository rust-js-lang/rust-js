# 1080. A value held only to be read twice, read once, is read in place

Status: Accepted. Amended: an `Option` its items hold is read in place
where it's read once (#588).

Case ([0262](0262-when-rust-and-js-disagree.md)): N. The same reads
happen, in the same order, either way.

## Context

Lowering holds a value it reads more than once in a `const` first, so
that it's read once (`spill`): an `Option`'s clone reads it in its test
and in its `Some` branch, `value == null ? value : value.slice()`, and an
`Option`'s items, `option == null ? [] : [option]`. Some of these readers
turn out to read it only once:

- `items.last().cloned()` of a type parameter's items, whose clone is the
  value itself, printed `const value = items.at(-1); return value;` (#587),
  found porting alien-signals. `items.first().cloned()` already printed
  `return items[0];`, because `items[0]` reads the same each time and was
  never held.
- `some.iter().count()` of an `Option` was the length of the array made of
  it, `(option == null ? [] : [option]).length` (#588).

## Decision

**An `Option`'s clone whose payload's clone is the payload itself is the
`Option` as it's written**: the `const` it was held in is taken back, and
its name given back (`clone_parts`), `return items.at(-1);`, and `(items)
=> items.at(-1)` of a closure. One whose payload needs a clone, a `Vec`'s,
reads it once, held, as before.

**An `Option`'s items counted are how many it holds, `o == null ? 0 : 1`**
(`eager`'s `count()`), not an array's length.

## Consequences

- dyn_any_generic's `map((item) => (item.impl.typeId() === TId ? item.value
  : undefined))` lost its `const value`.
- #588 isn't fully fixed. The count's `Option` is still held first,
  `const option = someOf(a);`, because `option_items` holds it before
  `count()` reads only its test, so the elements of a tuple before it are
  held in `const`s too, as before.

Held by corpus `cloned_option_of_a_type_parameter` and
`option_iter_count_in_a_tuple`, and the mutations `cloned-option-held-first`
(`lower/std_impls.ts`) and `option-count-of-its-array`
(`lower/iterators/eager.ts`).

## Amendment: an `Option` its items hold is read in place where it's read once (#588, 2026-11-04)

The count's `Option` was still held first, `const option = someOf(a);`,
so a tuple's elements before it were held in `const`s, `const isSome =
..; const value = ..; return [isSome, value, option == null ? 0 : 1];`.
**What `option_items` holds an `Option` in is a temporary of MIR's
lowering's** (`Made::fixed`, as `fixed_place_of`'s are), which tidy's
`inlined` reads in place where it's read once, as the count reads it:
`return [lastOf([2]) != null, lastOf([1]) != null, someOf([1, 2]) == null
? 0 : 1];`. Its items, read twice, `option == null ? [] : [option]`, keep
it held. Case ([0262](0262-when-rust-and-js-disagree.md)): N, the same
calls in the same order (`moved`'s rule of what may run before a read).

Held by the corpus `option_iter_count_in_a_tuple.rs`, now its elements in
place, against native Rust, and the mutation `option-items-held-apart`
(`lower/std_types/option.ts`). Closes #588.
