# 0392. A writer of bytes, `io::Write`, generic and `dyn`

Status: Accepted, amended by [0462](0462-io-error-values.md): an
`io::Error` is a value. Amends [0132](0132-std-odds.md); extends
[0180](0180-generic-writers.md), [0141](0141-std-trait-objects.md) and
[0108](0108-generic-operators-and-into.md). Amended: a writer of bytes of
the crate's, `impl io::Write`, arrayvec's `ArrayVec<u8, CAP>`, has a
dictionary too: its `write` and `flush`, and std's `write_all` and
`write_fmt`, `$ioWriteAll(write)` and `$ioWriteFmt(write)`, its `write` till
all's written, as `fmt::Write`'s defaults are made of its `write_str`. An
`io::Error` is a type it names, `io::Result<usize>`, of no value: no write
of rust-js's fails, and one of the crate's that writes none, or fails, is
rust-js's error where it does (case D), as an `io::Error`'s `{:?}` is a
compile error. A write to one of known type, `write!(v, ..)` of an
`ArrayVec<u8, 16>`, is its dictionary's, `Std::UserIoWrite`; and a write's
result's `unwrap_or(())` is `()` (`test/corpus/io_write_impl.rs`).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): the bytes written are
the program's output, so they're Rust's, as they are.

## Context

Code that writes output takes a writer of bytes, `W: io::Write` or `&mut
dyn Write`, and is given standard output, a buffer, or a box of either:

```rust
fn render<W: Write>(out: &mut W, n: u32) -> io::Result<()> {
    writeln!(out, "n = {n}")?;
    out.write_all(b"bytes\n")
}
fn shout(out: &mut dyn Write, text: &str) { .. }
```

ADR 0132 wrote to a standard stream whose type was known; a generic or
`dyn` writer, `write_all`, `write`, and a `Vec<u8>` as a writer were
errors. Five of rustc's tests failed on them. `ok()` of the `io::Result<()>`
a write gives, which ADR 0132 holds as nothing, read a `TAG` it hasn't: a
miscompile.

## Decision

**A `W: io::Write` is given a dictionary, as a `W: fmt::Write` is (ADR
0180), of `write`, `write_all`, `write_fmt` and `flush`; a `dyn io::Write`
is the pair of the writer and it, as a `dyn Error` is (ADR 0141). Where the
writer's type is known, the write is in place.**

| Writer | Its dictionary | Known, in place |
|---|---|---|
| `Vec<u8>` | `$vecWriter`: bytes added to the box's `value` | `write!` is `$writeText(v, text)`, `write_all` `$extend(v, bytes)` |
| `Stdout`, `Stderr`, their locks | `$stdoutWriter`, `$stderrWriter` | `write!` is `print!`'s (ADR 0132), `write_all` `$printBytes(bytes, 0)`, `write` `Ok` of its count |
| `Box<dyn Write>` | `$dynWriter`: through the pair it holds | |

```js
function render(out, n, WWrite) {
  WWrite.write_fmt(out, `n = ${n}\n`);
  return WWrite.write_all(out, [98, 121, 116, 101, 115, 10]);
}
shout({ value: buf, impl: $vecWriter }, "héllo");
```

- **Bytes to a stream are written as they are**: Node's stream is given
  them, so one that's no text, `0xFF`, is that byte, as Rust's is. Where
  there's no `process`, they're decoded as text, a character split between
  two writes kept till its end comes, and printed as `print!` prints.
- **A write never fails** (ADR 0132): `write` is `Ok` of all it's given,
  and the others nothing. The methods of a write's result, `fmt::Result`'s
  and `io::Result<()>`'s alike, are answered by `fmt_result_values`, one
  place: `unwrap()` `()`, `is_ok()` `true`, and now `ok()` `Some(())`.
- **Which writer a type is, is recognition's `io_writer`**, asked by the
  dictionary and by what's in place; `classify_io_write` says what's in
  place, out of the dispatcher.
- **A `dyn` method's receiver made where it's called, `{ value, impl }`,
  is held**, as one with effects was: it's read twice.

## Why

- **It's the program's output, byte for byte**: the corpus's
  `io_writers.rs` writes through a generic writer, a `dyn` one and a
  `Box<dyn Write>`, to a buffer, to standard output and to standard error,
  text that's not ASCII, a character split between two writes and a byte
  that's no text, compared with native Rust; four rustc tests pass.
- **It's ADR 0180's and ADR 0141's**: a dictionary for generic code, a
  pair for a `dyn`, the writer in a box.

## Consequences

- A writer of the crate's own, `impl io::Write for MyWriter`, `io::Cursor`,
  and `write` of a known `Vec<u8>` are still errors.
- A stream held in a variable is `undefined` held, `const err = undefined`,
  and one given to a generic writer is boxed and copied back, which does
  nothing: a stream holds nothing, as `()` doesn't, but isn't left out as
  `()` is yet.

## Amendment: an `io::Write` given by `&mut`

`W = &mut Vec<u8>` of an `io::Write` got the `Vec`'s own dictionary,
`$vecWriter`, as `io_writer` takes a `&mut` off, and generic code, which
boxes its `&mut self` again, wrote to the box: "w.value.push is not a
function" (#130); and `&mut W` of a generic `W` was refused. A `&mut`
writer's dictionary is `$mutWriter` of its writer's, of an `io::Write`
as of a `fmt::Write` (ADR 0454), one rule of both, and `$mutWriter`
writes bytes too. Held by `test/corpus/io_write_mut_generic.rs` and the
mutations `io-writer-mut-as-its-writer` and `mut-writer-writes-no-bytes`.
