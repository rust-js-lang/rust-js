# 0446. A `dyn fmt::Write` is a pair, and `fmt::write` is `write_fmt`

Status: Accepted. Extends [0049](0049-traits-and-generics.md),
[0166](0166-user-fmt-write.md), [0180](0180-generic-writers.md) and
[0392](0392-a-writer-of-bytes.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)): the same writes, to
the same writer.

## Context

A function that writes to any writer takes `&mut dyn fmt::Write`, as
`render(out: &mut dyn fmt::Write)` does, and std's `fmt::write(w, args)`
takes one too: anyhow's `ensure!` renders each side of a comparison with
it. A generic `W: fmt::Write` had a dictionary (ADR 0180), and a `dyn
io::Write` was a pair (ADR 0392), but a `dyn fmt::Write` was refused, a
value of no type rust-js knew, and `fmt::write` with it.

## Decision

**`fmt::Write` is one of std's traits whose `dyn` is a pair, its value
and the dictionary generic code already has: a `String`'s, `$stringWriter`,
given it in a box, and the crate's writer's own. `fmt::write(w, args)` is
lowered as the call it is, `w.write_fmt(args)`.**

```rust
fn put(w: &mut dyn fmt::Write, n: u32) -> fmt::Result {
    write!(w, "<{n}>")
}
```

```js
function put(w, n) {
  return w.impl.write_fmt(w, `<${n}>`);
}
```

- One function answers what call a std function is, `Recognition::call_it_is`,
  which the MIR lowering asks before anything else of a call.
- A writer of the crate's is given each `write_fmt`'s text in one
  `write_str`, as it is through `write!` (ADR 0166): the one listed
  difference from native Rust, 13, where one fails part way through.

## Why

- **It's Rust's**: a writer taken as a `dyn` is how a function writes to a
  string, a file or a buffer of the caller's choosing, and how anyhow
  renders its messages.
- **One form**: the `dyn`'s dictionary is generic code's, and `fmt::write`
  is the `write_fmt` it's written as.
- **It's tested**: a corpus case writes through `fmt::write` and a `dyn
  fmt::Write` to a `String` and to the crate's writer that fails, against
  native Rust; a mutation of each is caught.

## Amendment (2026-11-14, #1697)

`fmt::write(w, args)` is not `w.write_fmt(args)`. It is std's own
`write_fmt` body, which writes each piece through the writer's
`write_str` and `write_char` and never calls its `write_fmt`. A writer
whose own `write_fmt` is `fmt::write(self, args)`, the usual way to
write one, called itself through its dictionary, `w.impl.writeFmt`, and
overflowed the stack (#1697). Case: results change ([0262](0262-when-rust-and-js-disagree.md)),
so Rust's is kept.

`fmt::write` is now classified `Std::UserWrite` itself, `Recognition::is_fmt_write`.
It is lowered as std's `write_fmt` of the `dyn fmt::Write` it takes
(`call_it_is` still names that writer):

- its text given to the writer's `write_str`, `w.impl.writeStr(w, text)`;
- or, where writes are followed a piece at a time (ADR 1135), each piece
  written into `$writerSink(w, writeStr, writeChar)`.

Neither form calls the writer's own `write_fmt`. Proof: test/corpus/fmt_write_function.rs
(its snapshot's `writeFmt` became `writeStr`), and a writer of the crate's
own whose `write_fmt` is `fmt::write(self, args)` in test/writer-sink-own-write-fmt.rs
and in a library pair, against native Rust in both builds. Mutation:
fmt-write-takes-own-write-fmt.
