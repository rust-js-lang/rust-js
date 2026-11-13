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
- **What isn't followed yet is refused, naming #1652.**

## Landing

Each step lands on main with its runtime helpers and their tests: the
runtime's unit tests, `test/writer-sink.test.ts` against native Rust in
both builds, and the formatting grid's `Log` column (test/format-grid.ts),
which writes into a writer that records each call. Each runs where
`RUST_JS_WRITES_PIECEWISE=1` is set. That switch is test-only, and the
lowering doesn't read it by default, so each step leaves the corpus
byte-identical. The last step removes the switch and makes this the one
way, once `crate-corpus --check` shows no crate doing worse. Then ADR
0166's and 0180's whole-text rules, and the semantics page's difference,
go.

1. Integers, strings, `char`s and `bool`s with every option (landed with
   this ADR).
2. A crate's own `Display` and `Debug` framed on the caller's sink, one
   model with `analysis::prints`' `through_sink`.
3. Floats by `flt2dec`'s parts.
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
