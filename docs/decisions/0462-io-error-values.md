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

## Amendment: an `io::Result<()>`'s `Err` is thrown, as a `fmt::Result`'s is

An `io::Result<()>` is nothing (ADR 0132), a write's result, as a
`fmt::Result` is, and its `Err` was refused. A `fmt::Result`'s is thrown
(ADR 0187); an `io::Result<()>`'s is now too, by the same machinery, one
way for both: `is_write_result` asks of either.

```js
function doubled(s) {
  try {
    const result = parse(s);
    if (result.TAG === "Err") {
      return result;
    }
    check(s);
    return { TAG: "Ok", _0: n * 2 };
  } catch (error) {
    return { TAG: "Err", _0: $writeCaught(error) };
  }
}
```

- **A write's error is a `$WriteError`**, a `$FmtError` or a `$IoError`:
  `$writeTry`, `$writeCheck`, `$writeExpect`, `$writeResult` and
  `$writeOf`, ADR 0187's and ADR 0461's helpers renamed, take either.
  `expect` of one panics with its error's `{:?}`.
- **`Err(e)` of one throws `e`**, `$ioThrow(e)`, and so does `?` of a
  value's `Result` into one, `parse(s)?`.
- **A `match` of one tests `undefined`**, and its `Err`'s field is the
  error: `variant_test` and `mir_field` say so of a write's result.
- **A function that gives a value's `Result`, and `?`s a write's result,
  catches the error at its end** and gives it as its `Err`, converted by
  its `From`: a `return` in its middle would skip MIR's drops, which a
  throw doesn't. `$writeCaught` throws anything else on, a panic as a
  panic.
- **A function's last value, a write's result that may hold its error,
  is thrown on**, as a writer's is (ADR 0461), so its callers' `?` sees
  it.
- **What may fail** is the analysis of ADR 0187's, of both: a function
  that makes an `Err` of either, `?`s a value's `Result` of a write's
  error, or calls what may; std's `write_all` of the crate's own writer
  may wherever any of the crate's may, and a call of it is caught where
  its result is a value.
- **std's `write_all` of a writer whose `write` fails gives its error**,
  an interrupted one tried again, and `Ok(0)` std's own, `Error { kind:
  WriteZero, message: "failed to write whole buffer" }`.

`test/corpus/io_unit_result.rs` and its `_expect` and `_panic` cases run
each against native Rust.

