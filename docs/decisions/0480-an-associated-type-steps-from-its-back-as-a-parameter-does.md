# 0480. An associated type steps from its back as a type parameter does

Status: Accepted. Extends [0429](0429-a-double-ended-iterator-knows-its-back.md),
[0456](0456-a-generic-exact-size-iterator.md) and
[0474](0474-an-iterator-another-bound-reads-is-given-as-it-is.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)): the same items, from
the same end.

## Context

A type parameter bound by `DoubleEndedIterator` or `ExactSizeIterator` is
a JS iterator that knows its back or its length, its caller's (ADRs 0429,
0456). An associated type a bound says is one, `I::IntoIter:
DoubleEndedIterator`, wasn't: `iterable.into_iter().rev()`, itertools'
`rev` and `partition`, was refused. So was a `Copied` of a slice's iterator
given as one, `[1, 2].iter().copied()`. And ADR 0474 gave a parameter as it
is where a bound has methods, `DoubleEndedIterator`'s too, which its JS
iterator answers.

## Decision

- **An associated type a bound says is double-ended, or exact-sized, is a
  JS iterator that knows its back, or its length**, as a type parameter's
  is: `extension_param`'s, of either.
- **A `Copied` of one is the one it copies**, whose JS iterator it is: it
  knows its back and its length where that does, and is given as that is.
- **An iterator's extension trait doesn't make a parameter given as it
  is** (ADR 0474): its JS iterator answers its methods.

```js
function rev_all(iterable) {
  return $iterator($intoIter(iterable), $nextBackSome, true);
}

last_even($iter([1, 2, 3, 4, 5]));
```

## Why

- **It's std's**: `associated_double_ended` reverses a generic iterator and an
  `I::IntoIter`, finds from the back of a `Copied`, and takes the last of an
  `Iterator + DoubleEndedIterator`, with native Rust.

## Consequences

- **What compiles grows**: stepping an associated type from its back.

## Amendment: the crate's iterator given for a `T` whose `T::IntoIter` is bound (2026-11-13, #1604)

Generic code holds a `T::IntoIter` a bound says is double-ended or
exact-sized as a JS iterator that knows its back or its length. Its caller
made one only of a parameter's own bounds, `I: DoubleEndedIterator`. So the
crate's own iterator given for a `T: IntoIterator` reached generic code as
a JS iterator that knew neither. `rev_first(Up(0, 3))`'s
`t.into_iter().rev().next()` threw `it.back is not a function`, where
native Rust gives `Some(3)`, and `len_of`'s `len()` threw
`it.len is not a function`.

- **The caller asks the bounds of `T::IntoIter`, the crate's iterator
  being its own `IntoIter`.** Where they say it's stepped from its back or
  asked its length, the iterator is made knowing them, as an `I` so bound
  is (`stepping.rs`).
- **The dictionary of that extension is the JS iterator's**:
  `{ next_back: $nextBack }` and `$foldBack`, or `{ len }`
  (`made_iterator_extension_dictionary`). It isn't the impl's, which
  would step the iterator as it is. `$foldBack` reaches the iterator's own
  `rfold` through the value and says whether that took it (#649). So std's
  compiled `Rev`, whose `fold` reaches the crate's `rfold`, doesn't drop
  the iterator again; without it, an iterator with a destructor was
  dropped twice. One entry for a JS iterator that knows
  its back or its length, `knowing_entry`, is what this dictionary and
  `iterator_extension_dictionary` both give.
- **Another's items know them as they're made**: a `Vec`'s, `$intoIter`'s.
- **A `T::IntoIter` held as it is, of a `T` another bound names**, an
  `impl Is<T>`'s (#966), is stepped from its back by its dictionaries, the
  JS iterator `given_js_iterator` makes of it, as `rev()` of it already
  was. Its `rfold`, `nth_back` and `rfind` (`back_step`) stepped the
  caller's own value as a JS iterator, and `rfold` threw
  `it[by] is not a function` on main too. The metamorphic net's
  `impl-owned` rewrite of this case found it.

`test/corpus/into_iter_bound_stepped_from_its_back.rs` holds `rev()`,
`len()`, `map(..).rev()`, `rev().count()`, `rev().last()`, `rfold` and
`rev().fold(..)` of one that writes its own `rfold` and drops, an
`impl Is<T>`'s `rfold`, and giving
`t.into_iter()` on to an `I: DoubleEndedIterator` or an
`I: ExactSizeIterator`, of the crate's iterator and of a `Vec`. It runs
against native Rust in both builds.
