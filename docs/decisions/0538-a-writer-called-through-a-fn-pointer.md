# 0538. A writer called through a `fn` pointer

Status: Accepted. Extends [0054](0054-display.md) and [0125](0125-function-values.md).

Case: D ([0262](0262-when-rust-and-js-disagree.md)) closed: what was
rust-js's own error compiles as Rust's does.

## Context

A writer gives its `Formatter` to another writer it calls, a function or
a closure, `with_word(word, f)`, which is what it writes (ADR 0054). One
it calls through a `fn` pointer it's given was refused, "this
`Formatter`", as itertools' `Format` calls the `fmt` it's given of each
item:

```rust
fn format(&self, f: &mut fmt::Formatter, cb: fn(&I::Item, &mut fmt::Formatter) -> fmt::Result) -> fmt::Result {
    // ..
    cb(&fst, f)?;
}

impl<I: Iterator> fmt::Display for Format<'_, I> where I::Item: fmt::Display {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.format(f, fmt::Display::fmt)
    }
}
```

And std's trait method given as a value, `fmt::Display::fmt` of a `u32`,
was a call of the dictionary made there:
`(arg0, arg1) => ({ fmt: (value, options) => .. }).fmt(arg0, arg1)`.

## Decision

**A call through a `fn` pointer that writes to a `Formatter` it's given
is what it writes, `f += cb(x, options)`**, as a writer callable's is: its
`Formatter` isn't given, its options are, where the crate's writers take
them. `recognition::pointer_formatter` answers which argument it is, of
the pointer's signature, for what's refused and what's lowered alike.

**A trait method as a value whose dictionary is made there is that
dictionary's function**, `(value, options) => $formatted(String(value),
options, true)`: a call of a dictionary made in place, an arrow, is the
arrow's, and an arrow called with exactly its parameters is the arrow.

## Why

`writer_fn_pointer` gives `Display::fmt`, `LowerHex::fmt` and the crate's
own writer as such a pointer, written plain, `{:x}`, `{:#x}`, padded, and
of no items, against native Rust; the padding is each item's, its
`Formatter`'s options.

## Consequences

- itertools' `Format` compiles; `FormatWith`, whose closure writes to the
  `Formatter` it captures, `|disp| disp.fmt(f)`, is still refused.
- Mutations refuse the call, lose what it writes, and keep the
  dictionary or the arrow around it.
