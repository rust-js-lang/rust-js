# 0461. A `Formatter` held is a handle on its writer's text, and a `fmt::Result` meets generic code as a `Result`

Status: Accepted. Extends [0054](0054-display.md),
[0099](0099-mut-references.md), [0180](0180-generic-writers.md) and
[0187](0187-fmt-error.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)): the same text, and
the same `Err` where it's made.

## Context

base64's `Display` hands its encoder the `Formatter` in a struct of its
own, which the encoder's generic code writes to through a trait whose
error is the impl's:

```rust
impl Display for Base64Display<'_, '_, E> {
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), fmt::Error> {
        let mut sink = FormatterSink { f: formatter };
        self.chunked_encoder.encode(self.bytes, &mut sink)
    }
}
```

A writer's `Formatter` was its text, a local string (ADR 0054), and any
other use of it was refused. Through generic code, a `fmt::Result`,
`undefined` or its error thrown (ADR 0187), met a `Result<(), S::Error>`,
`{ TAG }`: the generic code read `undefined.TAG` and failed, as a `String`
sink whose `Error` is `fmt::Error` did already. And a writer's last call,
`check(n)` of a `fmt::Result` that may fail, gave `Ok`.

## Decision

**A `&mut Formatter` held anywhere but its writer is a handle on the
writer's text, as a `&mut String` is (ADR 0099), and its writes are a
`String`'s through it.**

```js
function showDisplay_fmt(show) {
  let f = "";
  const sink = { f: { get value() { return f; }, set value(value) { f = value; } } };
  ..
}

function formatterSinkSink_write_encoded(formatterSink, encoded) {
  formatterSink.f.value += encoded;
}
```

1. **A `Formatter` is its text, a string**, `is_std_wrapper`'s, as a
   `String` is: a `&mut` to one is a cell.
2. **A struct that holds the writer's `Formatter`**, `FormatterSink { f
   }`, holds a handle on its text.
3. **`f.write_str(s)`, `write!(f, ..)` and `f.write_char(c)` of a held
   one** are a `String`'s, `formatter_write`: `f.value += s`.

**A `fmt::Result` is a `Result` where generic code has one, and the
`fmt::Result` it is where it's read again, `is_generic_fmt_result`.**

- **An impl's, given through a dictionary whose trait method gives
  `Result<(), Self::Error>`**, is `$writeResult(() => ..)`: `{ TAG: "Ok"
  }`, or `Err` of the error thrown.
- **A generic function's `Result<.., S::Error>` read where it's a
  `fmt::Result`**, `generic_result_values`'s, is `$writeOf(..)`: `undefined`,
  or its error.
- **A call of one may fail where any of the crate's may**: its
  dictionary's calls give its error, so the analysis of what fails takes
  it as a dictionary's.

### Also here

**A writer's last call, its `fmt::Result` returned, is checked**, as
`?` of it is: `check(n)` throws where it fails, which the writer passes
on, `$writeCheck($writeOf(encode(..)))` of a generic one. It gave `Ok`.

## Why

- **It's the representation each already has**: a `&mut` to a string a
  handle on it, a `fmt::Result` in concrete code and a `Result` in
  generic code. Each boundary converts, where `generic_result_values` and
  the dictionary's entry already convert a generic `&mut T` (ADR 0454).
- **It's exact**: `test/corpus/formatter_sink.rs` writes through a
  `FormatterSink` and a `String` sink of `fmt::Error`, fails in each, and
  fails in a writer's last call, against native Rust; a mutation of each
  part is caught.

## Consequences

- A held `Formatter`'s options, `f.width()` or `f.pad(s)` through the
  struct, aren't the handle's: each is refused.

## Amendment: a `fmt` trait's method called on a held `Formatter`

`Display::fmt(&x, self.0)`, `Debug::fmt` or another `fmt` trait's,
called as a function on a `Formatter` held in a struct (2026-10-21), as
anyhow's `Quoted` writer escapes what it's given into the one it holds:
what the method writes, appended to the handle's text, `self.0.value +=
..`, as `self.0.write_str(s)` is. Plain, as the handle keeps no options
(above). `recognition::formatter_fmt_to` names it, `Std::FmtTo`;
`std_calls.rs` writes it beside `push_str`. The corpus's
`fmt_to_held_formatter.rs` holds anyhow's `Quoted` and a generic `show`
through a held `Formatter` against native Rust.
