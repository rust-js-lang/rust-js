# 0486. An `Option` of what could look like `None` is changed in place

Status: Accepted. (Amended by [0921](0921-an-option-slot-gives-its-iterator.md): what a `&mut` to is no cell, a generic iterator, is given as itself.) Extends [0051](0051-generic-options.md) and
[0326](0326-option-and-result-methods.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what's held, and what
a `&mut` to it writes, as std's.

## Context

An `Option` whose `Some` could look like `None`, an `Option<Option<T>>` or
an `Option<T>` of a type parameter, holds its `Some` boxed, `$some(value)`
(ADR 0051). Its methods that change its place, `get_or_insert`,
`get_or_insert_with`, `get_or_insert_default`, `insert` and `take_if` (ADR
0326), were refused of one: they'd have held what they're given unboxed, and
given back a `&mut` to the box. itertools' `Product` caches its current item
so, `a_cur.get_or_insert_with(|| a.next())`.

## Decision

**What they're given is held boxed, `$some(value)`, and the `&mut` they give
back is a handle through the box, `$someMut(handle)`**, which reads
`$someValue(o)` and writes `o = $some(v)`, as a `&mut` of an `Option`'s
`Some` is already (ADR 0051). `take_if`'s taken `Option` is given as it's
held, box and all, as `take`'s is.

- **Read where it's made, it's what the box holds**: `$someMut(h).value` is
  `$someValue(o)`, as a handle's `value` is its place (ADR 0099).
- **Written where it's made, the `Option` is given a `Some`**: `*o.insert(None)
  = Some(4)` is `o = 4`, as `$some(4)` is `4`.
- **`$someMut` is pure**: it only makes a value of what it's given, so one
  nothing reads isn't written.

```js
product.a_cur ??= $some(IIterator.next(a));
const value = $someValue(product.a_cur);
```

## Why

- **It's std's**: `boxed_option_places` changes `Option<Option<T>>`s in
  place, of a type parameter's items too, with native Rust.

## Consequences

- **What compiles grows**: each of an `Option`'s place-changing methods, of
  an `Option` of what could look like `None`.
