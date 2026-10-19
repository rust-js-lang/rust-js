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

- **It's std's**: `generic_double_ended` reverses a generic iterator and an
  `I::IntoIter`, finds from the back of a `Copied`, and takes the last of an
  `Iterator + DoubleEndedIterator`, with native Rust.

## Consequences

- **What compiles grows**: stepping an associated type from its back.
