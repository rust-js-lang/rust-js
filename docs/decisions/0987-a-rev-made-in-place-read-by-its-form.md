# 0987. A `rev()` made in place is read by its form

Status: Accepted. Corrects [0986](0986-len-of-an-iterator-made-in-place.md),
whose `as_exact_size` arm stepped a reversed range forward. It also closes a
gap: a `rev()` made in place, given where generic code steps it from its
back, was refused.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change. One
form gave a wrong answer; the others were valid Rust refused.

## Context

ADR 0986 made a `rev()` made in place knowing its length where generic code
asks it. Among its forms it read any map of what it reverses as its
source's map:

```rust
fn first<I: ExactSizeIterator<Item = u32>>(mut it: I) -> Option<u32> { it.next() }
first((1..9u32).map(|x| x + 1).rev())
```

The recognizers make this `$iterator(range, $rangeNextBack).map(f)`, a
range stepped from its end. Read as its source's `Map<Range>`, its range
was taken out of its `$iterator` and stepped forward again: `Some(2)`
where Rust gives `Some(9)`. Before 0986 it was refused. A map over an
array's items reversed where they're made, `$iter(xs.toReversed()).map(f)`,
is safe to read as its source's: the array itself is reversed.

`as_double_ended`, which makes what generic code steps from its back, had
no reading of a `rev()` made in place at all. `ends((1..4).rev())`,
`ends(v.iter().map(f).rev())`, `ends((1..9).skip(2).rev())` and
`ends((0..3).rev().rev())` were each refused.

## Decision

`reversal`, in `lower/iterators.rs` beside the other readers of the
recognizers' forms, is the one reader of a `rev()` made in place, as
`rev_items` and `reversed_inward` make it:

- `Stepped(it)`: `$rev(it)`, `it` made knowing its back where it was made;
- `Range(range)`: a range stepped from its end, `$iterator(range,
  $rangeNextBack)`, of a source that's a range, by its back step's name;
- `Items`: an array's items reversed, `$iter(xs.toReversed())`, which knows
  its back and its length;
- `Chain`: a map or a filter of such items, read as its source's type.

A chain over a range stepped from its end is none of these, and is refused.

`as_exact_size` and `as_double_ended` each ask `reversal`:
- `Stepped`: `as_exact_size` gives `$rev` of `it` made knowing its length;
  `as_double_ended` gives it as it is;
- `Range`: `$rev` of the range made knowing its back and its length;
- `Items`: as it is;
- `Chain`: read as its source's type.

`as_double_ended` asks before it takes a range out of its `$iterator`,
which would lose the reversal.

## Consequences

`ends((1..4).rev())` is `ends($rev($iterator({ start: 1, end: 4 },
$rangeNext, false, $rangeNextBack)))`. `ends((0..3).rev().rev())` is the
range stepped forward, knowing its back.

The diagnostics test "a map of a reversed range given where its length is
asked" holds the refusal, with the mutation
`reversal-of-range-chain-as-items`. `test/corpus/rev_given_to_generic_code.rs`
checks each form against native Rust, with the mutations
`double-ended-rev-unread` and `double-ended-reversed-range-forward`.
`exact-size-rev-unread` and `exact-size-reversed-items-unread` hold
`as_exact_size`'s reading.

Still refused: `$rev($skipping(..))` given where its length is asked,
`first((0..10).skip(6).rev())`. Not every `$skipping` is
`as_exact_size`'s, made knowing its length: `closing.rs` makes one too.
