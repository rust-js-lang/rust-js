# 1130. A line is formatted through stdout's line writer where formatting may print

Status: Accepted (2026-11-12, #1490). The user decided to preserve the
order (option A on the issue), and the coordinator approved the design.
Beside [0087](0087-printing.md), printing, and
[0054](0054-writers.md), writers.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's order is preserved.

## Context

Rust's stdout is a `LineWriter`. `println!` writes each piece into it as
it formats, and the buffer is written out at each newline. So when a
writer prints while it formats (a `println!` in a `Display`, or a drop
that prints in a `Debug`), the print lands inside the line formatted so
far: `println!("a {} b", Noisy)` prints `a inside` on one line, then
`<1> b`. Text a writer writes before its print comes before it:
`Grid`'s `{:?}` writes `[Loud(0), Loud(1)]` and then drops its cells,
which print, giving `[Loud(0), Loud(1)]drop 0`. Standard error is
unbuffered, so it is written at once, before the line it interrupts.

rust-js built each line as one string and wrote it with `console.log`.
Every print made while formatting came before the line. The metamorphic
net's `loud` rewrite found this as #1490. A buffer at `println!` alone
doesn't fix the loud case, because a writer gives back its whole string,
and the drops it runs have already printed by then.

## Decision

**One owner, `analysis/prints.rs`, answers which writers may print.** A
writer is a function or closure given a `Formatter`. It may print when
what it may run (`may_run`) reaches a body that prints to stdout
(`print!`, `println!`, or `io::Write` of `io::stdout()`,
`Recognition::writes_stdout`), or when that can't be followed. A writer
that shows a value of a type one of whose writers may print is
included, to a fixed point: a derived `Debug` reaches its fields'
writers through std's `&dyn Debug`, which `may_run` doesn't follow. The
owner also gives the bodies that may run while a line is formatted: those
writers and what each may run.

**Where formatting may print:**

- `println!` and `print!` write each piece through stdout's line as it's
  made: `$printing(() => { $out("a "); $out(noisyDisplay_fmt(n)); $out(" b\n"); })`.
  Of what may fail (ADR 0187), the pieces are written in a `try`, whose
  `$fmtPrintFailed` writes what was written before the failure into the
  line.
- A writer that may print writes its text through a sink:
  `const f = $sink(); f.write(..); return f.done();`. Inside `$printing`,
  the sink writes into the line and `done()` gives `""`. Anywhere else,
  the sink is a string of the writer's own, which `done()` gives back.
  The writer's own `write!` of such a value is written piece by piece.
- A print made by a body that may run while a line is formatted goes
  through the line, `$out(text)`, so it lands inside the line. `eprint!`
  is written at once and flushes nothing of stdout's line.
- A writer's text made while a line is formatted, for anything but that
  line (`format!`, `to_string()`, `write!` to a `String`), is made apart
  from it: `$formatting(() => text)`.

Everywhere else `println!` stays `console.log` and a writer still gives
back its string. Nine corpus snapshots changed, each one a writer that
prints.

**The runtime, print.js.** `$out` appends to the held line and writes
everything up to the last newline at once through `$print`: Node's
`process.stdout.write`, or, where there's no stream, `console.log` of
each line. When the outermost `$printing` ends, what's left unended (a
`print!`'s, or a panic's part-way through) is written as `print!` writes
it, so what comes next follows it: `before halfsaid`, then the next line.
In a browser, the line buffer flushes to `console.log` at each newline,
and a partial line is flushed at the end of the program's task.

**Refused:** a derived `Debug` or a `Debug` builder's step of a value of
the crate's whose `Debug` may print (#1495). Each field is shown whole
before the struct's text is written, and streaming it would also need
`{:#?}`'s indentation of nested output (`PadAdapter`). A `Formatter` held
where it isn't its writer's, by a writer that prints, is refused too.

## Proof

Corpus case `printed_while_a_line_is_formatted`, run against native Rust
in both builds. Its grid covers:
- a print inside a writer, after text the writer has written;
- a drop that prints, run by a `Debug` after its text;
- a writer that prints, written inside another writer's `write!`;
- `format!` and `to_string()` of one, inside a writer;
- `print!` part-way through, then `writeln!` to `io::stdout()` inside a
  writer;
- `print!` of one with no newline, ended by the next line;
- generic code's `println!` of a `T: Display` given one;
- a writer that prints and then panics, caught.

stderr is compared too: each `eprintln!` lands where Rust's does. The
metamorphic net's `loud` rewrites of `debug_hex_of_values_a_writer_makes`
and `pretty_debug_of_values_a_writer_makes` now run as native Rust, and
are off test/metamorphic-known.txt.

## Consequences

- A writer that panics part-way through a line but prints nothing still
  loses what it wrote before the panic, where Rust's line buffer keeps
  it. Covering that means treating "may panic" like "may print", which
  reaches most writers: #1496 asks the user.
- A library's writer that prints, shown by a consumer's `println!`, is
  shown as before, its string whole. No contract fact says it prints.
- Placing the panic message relative to a partial line, in a stream that
  mixes stdout and stderr, isn't modelled: rust-js writes the partial line
  when `$printing` ends, where Rust writes it with the next newline.
