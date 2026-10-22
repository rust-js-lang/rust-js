# 0620. An `Option` a test says is there is written as it is

Status: Accepted. Extends [0578](0578-a-nullable-result-where-its-written.md)
and [0564](0564-a-std-body-is-tidied-as-the-apps-are.md)'s decided tests.

Case ([0262](0262-when-rust-and-js-disagree.md)): A: what's written, not
what it does. `x ?? d` is `x` wherever `x` isn't `null` or `undefined`.

## Context

React's `getIteratorFn` returns the iterator method it finds, or `null`:

```rust
#[cfg_attr(rust_js, rust_js::nullable)]
pub fn get_iterator_fn(maybe_iterable: Option<&'static Unknown>) -> Option<&'static Unknown> {
    // ..
    if js::type_of(maybe_iterator) == "function" {
        return maybe_iterator;
    }
    None
}
```

A `#[rust_js::nullable]` result of another `Option` is `null` for
`undefined` where it's written (ADR 0578), so rust-js wrote `return
maybeIterator ?? null;` under `if (typeof maybeIterator === 'function')`,
where upstream returns `maybeIterator`: what the test says is a function
is never `null` or `undefined`.

## Decision

**Where a test is known to hold, or to fail, an `x ?? d` of a variable
it says is there is `x`**: `typeof x === "function"`, of any kind but
`"object"` and `"undefined"`, a test of `x` against a constant, or `x !=
null`, holding; `x == null` failing, after `if (x == null) { return null;
}`. It's what `decided_tests` knows already, the test of an `if` that
holds through its `then` and fails through its `else` and what follows a
branch that leaves (ADR 0564), up to what may change what it reads; now
it decides a `??` there too (`present_in`), of a variable nothing
assigns.

```js
export function callable(value) {
  if (typeof value === "function") {
    return value;
  }
  return null;
}
```

`implies` answers whether one test says another, and gains its one
missing case, `x != null` saying `x != null`.

## Why

- **A value's JS is what it is where it's read**: `?? null` says the
  value may be `undefined`, which the test it's under says it can't be.
- **One place knows what a test decides**, `decided_tests`: a `??` it
  decides is the same knowledge as an `if` it decides.

## Alternatives

- **Not write `?? null` where the result is written** (`nullable_written`):
  lowering doesn't know the tests a value is under. Rejected for asking
  of a value what only the code around it knows.
- **Write the Rust another way**, `maybe_iterator.filter(..)`: what's
  natural is the test upstream writes, and rust-js is what changes.

## Consequences

- A `??` of a variable inside a closure isn't one this decides: none is
  there yet when it runs, and the test of `shadowed` holds that a
  closure's own `value ?? 7` stays.
- Only a variable's `??`: one of a field, `a.b ?? d`, stays.

## What holds it

- `test/lowering.test.ts`, "an Option a test says is there is written as
  it is": `callable`, `logged` after a failed `== null`, and a closure's
  `(value) => value ?? 7`, run in Node; and "a nullable result is written
  where it's made", whose `if (flag && first != null) { return first; }`
  had `?? null`.
- Mutations `present-coalesce-kept`, `present-coalesce-failed-test-unread`.
