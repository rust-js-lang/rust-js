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
  `prints`' `framed` set. A writer is framed when a `write!` into a writer
  shows its type: the types are taken from the `format_args!` that write
  is given, not from the rest of its body. So are the writers a framed
  writer shows, makes a `&dyn Debug` of, or gives its `Formatter` to. Only
  a framed writer writes std's pieces; one that writes through a sink only
  for printing makes its text as before. For example:
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
- **What isn't followed yet is refused, naming #1652**: a map's or a
  set's `Debug`, a derived `Debug` of more than five fields, a write
  through a `Formatter` a struct holds, a closure or `fn` pointer given the
  `Formatter`, a `dyn`'s `Debug`, and an `Arguments` made elsewhere. A
  library's type or `dyn` whose contract doesn't say its dictionary
  carries `fmtInto` is refused naming #1688 (step 5).

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
   std's `PadAdapter` (landed). A string's `Debug` writes its quote by
   `write_char`, each run of `char`s shown as they are by one `write_str`,
   each escape by one, and the rest by one, even `""`; a `char`'s writes
   its quotes and itself by `write_char`, or its escape. Neither pads.
   The builders keep one model, the runtime's `debug_builders.js` of #1533:
   each step is std's `field` of a text, which the printing writers of
   #1533 and #1561 give it, or its `field_with`, `fieldWith(name, value,
   write)`, of a writing function that writes the value into the step's
   sink. For `{:#?}` that sink is `$padAdapter`, which does exactly what
   std's `PadAdapter` does: each line is one write, the four spaces before
   a line's start are another, and an empty write writes nothing. A
   `finish_non_exhaustive`'s `..` goes through it too. A std composite's
   `Debug` is written by those builders (`display::debug_sink_writes`):
   a sequence's list, an `Option`'s or a `Result`'s derived tuple, a
   tuple's `debug_tuple("")`, with a `Box` or a reference showing what it
   holds. The `Formatter`'s options are given to each part. A derived
   `Debug` (`debug_struct_field2_finish` and the rest) and a builder in a
   framed writer are written so too. A type a framed writer makes a
   `&dyn Debug` of is framed, and a generic type's writer frames what the
   types it's shown as give, not every writer. Proof: the `Debug` cases of
   `test/writer-sink.rs` against native Rust in both builds, the runtime's
   unit tests of the adapter and the steps, and the grid's `Log` column,
   now 1,368 cases, every value but a map's, a set's and a generic field's.
5. A generic or `dyn` value's `fmt` through its dictionary into the sink
   (landed). Each formatting dictionary, `Display`'s, `Debug`'s, a
   radix's, `LowerExp`'s and `UpperExp`'s, may carry one more entry,
   `fmtInto(f, x, options)`, which writes `x` into the sink `f` a piece at
   a time, as std's `fmt` of it does: of an impl of the crate's own,
   `$writingInto(f, pointDisplay_fmt, x, options)`, a generic impl's
   writer given its dictionaries after, `$writingInto(f, duoDisplay_fmt,
   x, options, TDisplay)`; of a std value's, its writes as `sink_writes`
   says, `fmtInto: $writePadded` where that's one helper of the value and
   its options; of std's errors made `dyn Error`s, the runtime's
   (`$stringError`, `$parseErrorDyn`, `$ioErrorDyn`'s `$writeIoError`,
   `$jsonErrorDyn`'s `$writeJsonError`). A `T` is written by its caller's
   dictionary, `TDisplay.fmtInto(f, t, options)`, and a pair `dyn`'s
   `Display` by its pair's, `d.impl.fmtInto(f, d.value, options)`.
   `analysis::written_params`, in `shown_params`' shape (#1331), says
   which dictionaries carry it: each function's type parameters it writes
   so, by its own `write!` into a writer, as a framed writer, or by what it
   gives them to, to a fixed point; the types that reach one; and, where a
   `dyn` is written so, crate-wide to start, every impl of the crate's own
   and every value it makes a `dyn` of. A dictionary nothing writes so
   carries none, so no other JS changes. A library's contract says it, as
   two `summary` facts and one `closed-world` one (ABI 35, ADR 1129): a
   parameter's `written_into`, of which a consumer's call gives the
   dictionaries with `fmtInto`, and which frames the writers of the
   consumer's types it gives; `Library.written_into`, its types whose
   dictionaries carry it; `dyn_written_into`, whether it writes a `dyn`
   it's given so, which makes the consumer's `dyn`s carry it; and
   `makes_fmt_dyns`. A library's type its contract doesn't list, written
   through generic code, and a `dyn` written so where a library makes
   ones that may not carry it, are refused, naming #1688; a library
   compiled before ABI 35 is refused by its version. A `dyn`'s `Debug`
   isn't followed yet, as std's errors' `Debug`s aren't, and stays
   refused (#1652). Proof: test/writer-sink-generic.rs against native Rust
   in both builds and its metamorphic rewrites (50, each the same), the
   formatting grid's `Pair<u8>` now in its `Log` column, three crate pairs
   (a library's generic writer given a consumer's type, a library's type
   written through a consumer's generic code, and one refused), the
   runtime's unit tests of the errors' writes, and 26 mutations.
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
