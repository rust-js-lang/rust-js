# 0453. `{:?}` of a slice's iterator is what's left of it

Status: Accepted. Extends [0071](0071-stepping-iterators.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)): std's text.

## Context

A struct that walks bytes holds a `slice::Iter<'_, u8>`, and derives
`Debug`: percent-encoding's `PercentDecode`, which url takes. `{:?}` of
one was refused. std shows what's left of it, `Iter([2, 3])`, as its
`as_slice()` gives it, and rust-js's slice iterator, `$iter(items)`, knows
where it is (ADR 0071).

## Decision

**`{:?}` of a `slice::Iter` is `Iter(` and its rest's `{:?}`, its
`as_slice()`, then `)`. What's left of a stepped iterator has one
function, `stepped_left`, which `as_slice()` asks too: of `$iter(v)` just
made, `v` (`fresh_iter_source`); else `it.items.slice(it.at, it.end)`,
which reads it twice, so one a call gives is read once, in an arrow's
parameter, which tidy makes a `const`.**

```rust
let mut it = data.iter();
it.next();
println!("{it:?}");
```

```js
const it = $iter(data);
$next(it);
console.log(`Iter([${it.items.slice(it.at, it.end).map((item) => String(item)).join(", ")}])`);
```

- A `slice::IterMut`'s, whose items are handles on their places, isn't
  yet, nor `{:#?}` of one: refused.

## Why

- **It's std's text**, in a derived `Debug` of what shared code holds.
- **One place answers what's left of an iterator**: `as_slice()`, `cycle()`
  and `{:?}` ask `stepped_left`, which asks `fresh_iter_source` whether
  it's just made; `as_slice()` wrote its own slice before.
- **It's tested**: a corpus case shows one stepped, one just made, one a
  struct holds and one a call gives, against native Rust; a mutation of
  each part is caught.
