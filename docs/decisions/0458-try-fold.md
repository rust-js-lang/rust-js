# 0458. `try_fold` stops at the first `None` or `Err`, leaving the rest to step

Status: Accepted. Extends [0055](0055-iterator.md) and
[0071](0071-stepping-iterators.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)): the same result, and
the same items left in the iterator.

## Context

`try_fold` is how Rust folds what can fail: a sum that may overflow,
`iter.try_fold(0, |sum, x| sum.checked_add(x))`, or texts that may not
parse, `?` in its closure. itertools counts combinations with it. It was
refused.

## Decision

**`it.try_fold(init, f)` into an `Option` is `$tryFoldSome(it, init, f)`,
and into a `Result` `$tryFoldOk(it, init, f)`: each steps `it` with its
`next()`, gives `f` what it's made so far and the item, and stops at the
first `None` or `Err`, which it gives; else what the last gave.**

```js
function checked_total(items) {
  return $tryFoldSome($iter(items), 0, (sum, x) => $checked(sum + x, 0, 4294967295));
}
```

1. **It closes nothing**: it steps with `next()`, not a `for` loop, whose
   early end would close `it`, so what's left is there to step, as
   Rust's is through a `&mut`. A `&mut` receiver isn't `$lent` for it.
2. **Its items are its closure's**, each moved to it, and what it leaves
   is the iterator's, dropped with it, as `find`'s are: generic code
   given items with a destructor drops them where Rust does.
3. **Into an `Option` whose `Some` is boxed**, of a `()` or an `Option`,
   it's refused yet; a `ControlFlow` is too.
   (Amended: an `Option` of what could look like `None` compiles,
   [ADR 0539](0539-a-closure-writes-its-makers-formatter.md).)

## Why

- **It's idiomatic Rust**: a fallible fold, written with `?`.
- **Two helpers, one for each kind of result**: `$tryFoldSome` and
  `$tryFoldOk` read as what they stop on, where one helper would take a
  flag.
- **It's tested**: a corpus case folds an `Option` and a `Result`, stops
  early through a `&mut` and steps what's left, folds a range, and folds
  items with a destructor in generic code, against native Rust; the
  refusal is in `diagnostics.test.ts`; a mutation of each part is caught.
