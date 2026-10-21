# 0539. A closure writes to its maker's `Formatter`, and `try_for_each`

Status: Accepted. Extends [0054](0054-display.md),
[0187](0187-fmt-error.md) and [0458](0458-try-fold.md).

Case: D ([0262](0262-when-rust-and-js-disagree.md)) closed, and C for
`try_fold` into an `Option` of what could look like `None`: its `Some`
apart from `None`, as Rust's.

## Context

itertools' `FormatWith` writes each item through a closure that writes to
the `Formatter` it captured, in a closure `try_for_each` calls, which
writes to it too:

```rust
iter.try_for_each(|elt| {
    if !self.sep.is_empty() {
        f.write_str(self.sep)?;
    }
    format(elt, &mut |disp: &dyn fmt::Display| disp.fmt(f))
})?;
```

Each was refused: a closure that captured its writer's `Formatter`, "this
`Formatter`", and `try_for_each`. Finding why crashed rust-js first:
whether `{}` of a value may fail walked into a `dyn for<'a> FnMut(&'a dyn
Display)` and asked of `&'a dyn Display`, a type only inside its binder.

## Decision

**A closure that captured its writer's `Formatter` is a writer whose
string is its maker's**, which JS's closure writes as its maker does: it
neither makes it nor gives it back, and its `Formatter`'s options are its
maker's.

```js
function eachDisplay_fmt(each, options) {
  let f = "";
  const show = (x) => {
    f += x.impl.fmt(x.value, options);
  };
  // ..
  return f;
}
```

- `writer::body_writer` answers which writer a body is, of a `Formatter`
  it's given or one its closure captured, and its locals that are it,
  through its environment's field; a capture it can't write is refused.
- **`try_for_each(f)` is std's `try_fold((), |(), x| f(x))`**, one way:
  `$tryFoldOk` or `$tryFoldSome` of nothing and `f` given what's made
  first. Into a `fmt::Result`, whose `Ok(())` is nothing and whose `Err`
  is thrown (ADR 0187), it's each item's call, `forEach`; `try_fold`
  into one is refused. `IterMethod::tries` is the one answer to which
  methods fold that may stop.
- **`try_fold` into an `Option` of what could look like `None`**,
  `Option<()>`'s, `try_for_each`'s: `$tryFoldSome` gives its function
  what's made unboxed and gives it back boxed, which of anything else is
  as it was. ADR 0458 refused it.
- **Whether a write may fail asks no type a binder holds**, `&'a dyn
  Display` of a `dyn for<'a> FnMut(&'a dyn Display)`.

## Why

- `closure_writes_captured_formatter` is `FormatWith`'s shape, its user's
  callback given such a closure as a `&mut dyn FnMut`, in a closure
  `try_for_each` calls, and a closure called in a loop, padded, against
  native Rust; without this, rust-js crashed, then refused it.
- `try_for_each` stops at the first `Err` and at the first `None` of an
  `Option<()>`, what's left still there to step, and `try_fold` into an
  `Option<Option<u32>>` gives `Some(None)` apart from `None`.

## Consequences

- itertools' `FormatWith` compiles.
- `diagnostics.test.ts` refuses `try_fold` into a `fmt::Result` where it
  refused `try_fold` into an `Option<()>`.
- Mutations make a closure's string its own, give it back, lose its
  options, refuse the closure, crash on the binder, refuse `try_for_each`,
  give its function what's made last, read a `fmt::Result` as a `Result`,
  refuse a boxed `Option`, and pass what's made boxed or give it unboxed.
