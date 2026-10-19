# 0462. An `io::Error` is a value: its kind, and the error it holds

Status: Accepted. Amends [0392](0392-a-writer-of-bytes.md); extends
[0141](0141-std-trait-objects.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)): the same kind, and
the same text shown.

## Context

Code that parses or reads reports what went wrong as an `io::Error`:
`io::Error::new(ErrorKind::InvalidData, e)`, `io::Error::other("too
big")`, `ErrorKind::NotFound.into()`, and asks it its `kind()`. base64's
reader makes them. An `io::Error` was a type of no value (ADR 0392): no
write of rust-js's fails, so each of these was refused.

## Decision

**An `io::Error` is a `$IoError`, a class, of its kind, an `ErrorKind`'s
name as a fieldless enum's value is, and the error it holds, the `dyn
Error` pair std's `Box<dyn Error + Send + Sync>` is (ADR 0141), where it
was made of one.**

```js
new $IoError("InvalidData", { value: result._0, impl: $parseErrorDyn("ParseIntError") })
new $IoError("NotFound")
```

- **`new(kind, error)` and `other(error)`** hold `error` as std holds it:
  a message's `$stringError()`, an error's own `Error` dictionary, one
  that's a `Box<dyn Error>` already as it is. `From<ErrorKind>`, and
  `into()` of a kind, hold none.
- **`e.kind()` is `e.kind`.**
- **`{}` is what its error shows, or its kind's words**, std's `as_str`,
  `$ioErrorDisplay`; `{}` of an `ErrorKind` is its words, `$ioKindText`.
- **`{:?}` is std's `Custom { kind: InvalidData, error: .. }`, its
  error's own `{:?}`, or `Kind(NotFound)`**, pretty where it's asked,
  `$ioErrorDebug`.
- **As a `dyn Error`, `$ioErrorDyn()`**, whose `source()` is its error's
  own, as std's is. A concrete one's `Error` methods ask the same
  dictionary: `std_error_dictionary` answers for each std error rust-js
  holds as its own value, a parse error's, serde_json's and an
  `io::Error`'s.
- **A class**, as a `fmt::Error` is (ADR 0187), so that one can be thrown
  and told apart from a panic.

## Why

- **It's std's**: the `io_error_values` corpus case makes each kind of
  `io::Error`, asks its kind, shows it plain and pretty, and boxes it as
  a `dyn Error`, against native Rust.
- **It's what JS writes**: an object of a class, of the two things std's
  holds.
