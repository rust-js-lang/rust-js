# 1135. A `write!` into a writer is written a piece at a time

Status: Accepted (2026-11-13, #1652, campaign #1681), landing in steps.
Tu's decision (option A, std's exact split); the lead approved the design.
It replaces [0166](0166-user-fmt-write.md) and the "one `write_fmt` of its
text" rule of [0180](0180-generic-writers.md) once the last step makes it
the one way; until then they stand.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change. std's
`write_fmt` calls a writer's `write_str` once per piece, and a writer that
counts its calls, or fails at one, behaves differently if it gets the text
whole. So Rust's split is kept, however the JS reads, and a value whose
split isn't followed yet is refused.

## Context

ADR 0166 gave a writer of the crate's own a `write!`'s text whole, one
`write_str`, and ADR 0180 did the same for a `W: fmt::Write`'s dictionary,
`writeFmt(w, text)`. std writes it a piece at a time (`fmt::write`): each
literal piece of the template is one `write_str`; each value's `fmt` writes
its own pieces through the `Formatter`. An integer's sign is a `write_char`,
then its digits are one `write_str`. Each fill char is a `write_char`. A
float is written by its parts, `"1"`, `"."`, `"5"`. A `str`'s `Debug` is
written by its escapes. The builders write each separator. #1652's writer
counted three calls natively and one in JS, and its second `write!` kept
`"ab-cd[a"` natively and `"ab-cd"` in JS.

## Decision

**A `write!`, `writeln!` or `fmt::write` into a writer of the crate's own,
a `W: fmt::Write`'s or a `dyn fmt::Write`'s writes through one sink,
`$writerSink(w, writeStr, writeChar)`, each piece as std makes it:**

```js
const f = $writerSink(c, countedWrite_writeStr, countedWrite_writeStr);
f.write(value);
f.write("-");
$writeInteger(f, n);
```

- **`display::sink_writes` is the one owner of std's split** of each value:
  `SinkWrite::Str` (one `write_str`), `SinkWrite::Char` (one `write_char`),
  or a runtime function that writes into the sink. These are std's
  `pad_integral` (`$writeIntegral`, `$writeInteger`) and `pad`
  (`$writePadded`), with each option: sign, `#` prefix (even `""`), zeros,
  fill by alignment, precision in `char`s.
- **`write_char` is the writer's own where it has one, else std's**, a
  `write_str` of the `char`. A generic writer's dictionary has the one it
  should use.
- **Each call throws the writer's `Err`**, as `?` passes it on, so nothing
  after it is written. A `write_fmt` whose `Result` is a value gets it from
  `$writeTry`.
- **A literal-only `Arguments` is one `write_str`**, as std's
  `as_statically_known_str` writes it, and so is an argument rustc folds
  into the template, `b{}` of `1`.
- **A writer of the crate's own shown into a writer writes into it too**,
  one model with `analysis::prints`' `through_sink` (step 2): it writes
  through a sink, and the `write!` calls it with that writer's sink armed,
  `$writingInto(f, pointDisplay_fmt, p, options)`. Its own `write!` is
  written by `sink_writes`; `write_str` is one call and `write_char` the
  writer's `write_char`; `pad`, `pad_integral` and a `fmt` it gives its
  `Formatter`, a std value's, write by its `Formatter`'s options as they
  are when it runs (`sink_writes_given`); a writer of the crate's own it
  shows, or gives its `Formatter` to, `helper(f, n)`, is armed the same
  way. A write whose `Result` it keeps is in `$writeTry`, as the sink's
  calls may throw the writer's `Err`, and one that returns its own `Err`
  ends the `write!` with it.
- **The armed sink belongs to the writer `$writingInto` calls, and to no
  other.** Its arguments are evaluated before it's armed, as JS evaluates
  a call's arguments before the call, and the writer's first statement is
  its `$sink()` (`writer_text_made`, pushed before anything else of its
  body), which claims and disarms it. So a `format!`, `to_string()` or
  `write!` into a `String` the writer makes, whatever claims a sink after,
  is a string of its own, written where the writer writes it, as Rust's
  is. A writer that never claimed it is rust-js's error, thrown, never its
  text written whole. `writes_into_a_writer` arms only a writer of the
  crate's own that writes through a sink; another crate's, made whole in
  its library, is refused.
- **What isn't followed yet is refused, naming #1652**: a
  `Debug` of text or of a composite, a builder or derived `Debug` in a
  framed writer, a write through a `Formatter` a struct holds, a closure
  or `fn` pointer given the `Formatter`, a `dyn` or generic value, and an
  `Arguments` made elsewhere.

## Landing

Each step lands on main with its runtime helpers and their tests: the
runtime's unit tests, `test/writer-sink.test.ts` against native Rust in
both builds, and the formatting grid's `Log` column (test/format-grid.ts),
which writes into a writer that records each call. Each runs where
`RUST_JS_WRITES_PIECEWISE=1` is set. That switch is transitional, not an
option. Only tests set it, and nothing reads it by default, so each step
leaves the corpus byte-identical. The final commit, which routes `write!`
through the sink, deletes it and every read of it, so rust-js never has
two modes. The last step removes the switch and makes this the one
way, once `crate-corpus --check` shows no crate doing worse. Then ADR
0166's and 0180's whole-text rules, and the semantics page's difference,
go.

1. Integers, strings, `char`s and `bool`s with every option (landed with
   this ADR).
2. A crate's own `Display` and `Debug` framed on the caller's sink, one
   model with `analysis::prints`' `through_sink` (landed: the framed cases
   of `test/writer-sink.rs`, the grid's own `Display` in its `Log` column).
3. Floats by `flt2dec`'s parts (landed): `$writeFloat` makes core's
   parts, `digits_to_dec_str`'s and `digits_to_exp_str`'s, of the shortest
   digits (`$f64Digits`, `$f32Digits`) or the exact ones, at most
   `estimate_max_buf_len` of them of the value's decoded exponent, the rest
   zeros, rounded half to even; and writes them as
   `pad_formatted_parts` does: the sign a `write_str`, an empty one where
   `{:0}` writes it before the zeros, each part one, a run of zeros 64 at a
   time, each fill char a `write_char`. Its proof: the floats of
   `test/writer-sink.rs`, `f32`s too, and the grid's floats in its `Log`
   column, against native Rust in both builds, and the runtime's unit test
   of each form.
4. `Debug` escapes of `str` and `char`, and std's builders, pretty ones by
   std's `PadAdapter`.
5. A generic or `dyn` value's `fmt` through its dictionary into the sink.
6. An `Arguments` passed on, a writing function, so either's and
   arrayvec's own `write_fmt` pass it on.

## Alternatives

- **Keep ADR 0166's whole text**, listed as a difference: rejected, since
  the results change (Tu, 2026-11-13).
- **Switch over at the first step, refusing what isn't followed yet**:
  rejected, because either, arrayvec, itertools, rust_decimal, anyhow, toml
  and toml_edit would go from compiling to refused.
- **One long branch**: rejected, since rebasing it onto a main that moves
  by a hundred commits a day costs more than landing each proven step.
