# 0992. `eq()` of two `TrustedLen` iterators asks their lengths first

Status: Accepted. Amends [0465](0465-dropping-an-adapter-is-closing-it.md)'s
`$iterEq`. A wrong answer fixed where both lengths are counted; the rest is
filed as #386. Found by #382's triage of
`compiled-iterator-of-destructors-not-dropped-after`.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change. Items
were compared, and dropped in pairs, where Rust compares none.

## Context

```rust
vec![D(1), D(2), D(3)].into_iter().eq(vec![D(9), D(2)])
```

std's `eq_by` calls `SpecIterEq::spec_iter_eq`. Its impl for two
`TrustedLen` iterators compares their size hints first. Of unequal lengths
it returns `false` with no item stepped or compared, and drops the other
side, then this one: `drop 9`, `drop 2`, then `drop 1`, `drop 2`,
`drop 3`. `$iterEq` stepped both in pairs, so its order was `drop 9`,
`drop 1`, … A side-effecting `PartialEq`, or a map stepped on either side,
ran where Rust's never does.

## Decision

`iterators_eq` asks whether both sides are `TrustedLen`, by
`trait_holds`. Where both are, and each can be counted before it's
stepped, it gives `$iterEq` their lengths:
- an iterator of an array's items, made here or held, where it is
  (`items_left`, `counted_before_stepping`);
- another made in place, as `len_in_place` counts it
  ([0986](0986-len-of-an-iterator-made-in-place.md));
- what's given whole, a `Vec` or an array, its `length`.

Of unequal lengths `$iterEq` gives `false` before stepping either. Its
`finally` closes the other side, then this one, as std drops them.

The facts module said a `TrustedLen` specialization outside the zip's
changes nothing a program sees. That's corrected: `SpecIterEq`'s does, and
`eq()`'s lowering asks it itself.

## Consequences

`test/corpus/eq_of_unequal_lengths.rs` checks a `Vec`'s, a slice's and a
range's iterators against native Rust. The mutations are
`iter-eq-lengths-unread` and `iter-eq-not-trusted`. `adapter_drops`' two
`eq()` calls now carry their lengths.

Still differing, filed as #386:
- a map or another adapter whose value is held in a MIR temporary, which
  isn't counted;
- generic code, whose `TrustedLen` only its caller knows;
- `TrustedLen` iterators that aren't `ExactSizeIterator`s, such as a
  `chain()`.

`compiled-iterator-of-destructors-not-dropped-after` survived because a
compiled struct's by-value consumers close it themselves. Its `return()`
after the consumer is kept as the net it is, and its mutation is checked
by `peekable_given_back_drops`' snapshot.
