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
the crate's whose `Debug` may print (#1495; lifted, amended below). Each field is shown whole
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

- A writer that panics part-way through a line but prints nothing loses
  what it wrote before the panic, where Rust's line buffer keeps it, only
  in a program that catches no panic (amended below, #1496).
- A library's writer that prints, shown by a consumer's `println!`, is
  shown as before, its string whole. No contract fact says it prints.
- Placing the panic message relative to a partial line, in a stream that
  mixes stdout and stderr, isn't modelled: rust-js writes the partial line
  when `$printing` ends, where Rust writes it with the next newline.

## Amendment: a writer's prints inside a std value's text (2026-11-12, #1521)

A writer that prints, shown inside a std value's `Debug`, a `Vec`'s, an
`Option`'s or a tuple's, wrote its text straight to the line through
`$sink()`. But the JS makes the value's text as one expression,
`v.map(dDebug_fmt).join(", ")`, before the line gets any of it. So each
item's text reached the line before the brackets and commas around it,
`vec [!D(1)!D(3)!D(5), , ]` where Rust writes `vec [!D(1), !D(3), !D(5)]`.
And `$out` with no line formatted held a `format!`'s writer's prints for a
line, so the `console.log` of the next line came first.

The fix is in the runtime alone (src/runtime/print.js):

- **In a line, a writer's sink is a capture.** `$sink()` opens one on a
  stack and `done()` closes it, giving back the writer's text. A print
  made while a capture is open, through `$out` (a writer's `print!`, or a
  nested `println!`'s pieces), goes into the innermost capture as a mark
  at that point. The printed text is kept in a table under the mark's
  number. When no capture is open, `$out` puts each mark's text back in
  its place, recursively, and the line gets exactly the bytes Rust's
  `LineWriter` holds, each print where it ran.
- **A mark is unforgeable.** It opens with a lone surrogate, which no Rust
  string holds, followed by eight characters drawn at random once per run
  (the lead's guard). A JS string given through interop may hold lone
  surrogates, `\uDC000\uDC01`, and still reaches the line as it is.
- **`{:#?}`'s indent goes where std's `PadAdapter` writes it.** `$pretty`
  indents a part through `$padded`, which puts four spaces before each
  line's first character of text. A mark at a line's start stays before
  them, since the print, made before the writer's first write, doesn't go
  through the adapter: `!    D(1),`.
- **A writer a panic or an error stopped keeps what it wrote.** Its
  capture, still open, is added after what it was opened in, by the next
  write of the writer it was opened in or by `$printing`'s end. So a
  writer that catches its part's panic writes on after the part's text,
  and a line a caught panic stopped keeps the writer's text and prints.
- **With no line formatted, `$out` prints at once,** as `print!` does.

One shape can't be rebuilt at run time: a panic or an error part-way
through a composed value, at the second item of a `Vec`. The first
item's closed text and the separators live only in the JS expression.
So a std value's `Debug` of a part whose writer prints and may stop
part-way, in a crate that calls `catch_unwind`, is refused, naming #1496
(`Prints::stops_composed`, the lead's decision). That writer is a
`Debug` writer that may unwind (not `never_unwinding`) or may return an
`Err` (`failing`). Without a `catch_unwind` the program ends on the
panic, ADR 0262's difference 57. `never_unwinding` takes a writer's
`write!` into its `Formatter` to possibly unwind, so a writer that
prints and uses `write!` is refused there even when it can't panic. That
is a refusal, never a wrong answer, and it's rare: a printing writer,
shown in a std value, in a crate that catches panics.

Corpus cases printing_writer_shown_in_std_values (the issue's `Vec`,
`Option`, tuple and `format!`, `{:>8?}`, nested `Vec`s and arrays, a
`BTreeMap`, `{:#?}` of each, a writer whose `println!` ends the line
inside a part, and one showing a `Vec` inside its own text) and
printing_writer_stopped_part_way_where_caught (a writer that prints and
panics, alone or inside another's text, caught; one that catches its
part's panic and writes on; one that never stops, in a `Vec`), each
against native Rust in both builds. test/runtime-print.test.ts covers a
JS string's lone surrogates beside a mark, and test/diagnostics.test.ts
the refusal. Mutations `print-in-a-writer-unmarked`,
`marks-reach-the-line`, `unframed-print-held`,
`stopped-writers-text-lost`, `stopped-writers-text-folded-late`,
`marks-forgeable`, `padded-print-after-the-indent`,
`composed-stopping-writer-unrefused` and
`composed-stopping-writer-refused-unasked`; `sink-never-the-line` and
`pretty-not-indented` are refitted.

## Amendment: a failing writer's text written into a `String` (2026-11-12, #1520)

A writer that prints and returns `Err`, given to `write!` into a
`String`, threw std's `to_string` panic instead of returning the `Err`.
A `format_args!` whose formatting may print is `Value::Printing`, which
`printing_call` turns into a text that may fail, `$writeTry`'s, only where
it's the call's first argument. A `write!`'s text comes after the place
it writes to, so the failure was dropped. Now it's found wherever it is
among the arguments. And the text, made apart from the line in a body
that runs while a line is formatted, is made apart value by value,
`buf += "<"; buf += $formatting(() => refusedDisplay_fmt())`, so the text
before the piece that fails is written, as ADR 0187 writes it.

Corpus case printing_writer_refused_into_a_string (the issue's program; a
writer that prints and succeeds, its `Ok`; and a writer that writes both
into a `String` of its own as it's shown in a line) against native Rust
in both builds. Mutations `printing-text-of-a-write-unseen` and
`printing-text-made-apart-whole`.

## Amendment: a writer's kept write result (2026-11-12, #1524)

A writer's `write!` into its own `Formatter` gave `undefined`, `Ok`, and
its error was thrown, which is right for `?`: the writer's own error
carries it out, and what the writer wrote before it is added by
`$fmtWritten`. But a result kept, `let r = write!(f, "{:?}", inner)`,
dropped, `let _ = ..`, or asked, `.is_err()`, was `Ok`, and the inner
error was thrown out of the writer where Rust gives `r = Err(fmt::Error)`
and writes on. Now a write of what may fail whose result is read other
than as the writer's own (`given_on`: the writer's result itself, or
read only by `Try::branch`, `?`'s) is the
write's result as a value, as a `String`'s is (ADR 0187): `$writeTry(() =>
{ try { f += inner } catch (error) { f += $fmtPartial(error); throw error;
} })`, what the inner writer wrote before its error kept in the writer's
text. `write!(f, ..)?`, and a writer's last `write!`, are as they were. This isn't about printing; the
amendment is here beside #1520's, the same writer-result area.

Corpus case write_result_kept_by_a_writer (a result kept, dropped and
asked; `?`; and a writer that prints, kept) against native Rust in both
builds. Mutations `kept-write-result-taken-ok`, `tried-write-result-held`
(held by the snapshot) and `kept-write-result-partial-lost`.

## Amendment: a derived `Debug`'s field and a builder's step of a writer that prints (2026-11-12, #1495)

A derived `Debug` of a value whose `Debug` may print, or a `Debug`
builder's step of one, was refused (this ADR's **Refused**): each field
is shown whole before the struct's text is written, where Rust's print
lands between. #1521's runtime makes that right with no change to the
lowering: the field's writer, called in a line, writes into a capture,
each print a mark where it ran; the struct's text carries the marks to
the line, which puts each print back in its place, and `$padded`
indents `{:#?}`'s nested lines after a mark at a line's start, as
`PadAdapter` does. So the refusal goes.

What stays refused is one shape, #1521's for a std value's `Debug`, now
one rule for both: a field or step whose writer may print and stop
part-way, by a panic or an `Err`, in a crate that calls `catch_unwind`.
The struct's text before that field lives only in the JS expression, so
the line would lose `Holder { first: !Grid(1), second: ` where Rust's
stdout keeps it. `Prints::stops_shown` answers it for the derived `Debug`
and the builder (`refuse_stopped_in_place`) and, of a std value's type,
`stops_composed`; both name #1530, which streaming a composed `Debug`
through the sink would lift.

Corpus case printing_writer_shown_by_derived_debugs_and_builders (a
struct, a tuple struct, an enum's variants, nested, `debug_struct`,
`debug_list().entries`, `debug_tuple`, `debug_set`, `debug_map`, each
`{:?}` and `{:#?}`, a `format!` of one, and one after a `print!`) against
native Rust in both builds. test/diagnostics.test.ts holds the refusal
of a derived `Debug` whose field may print and panic where it's caught.
Mutations `stopped-in-place-unrefused` (refitted from
`printed-in-place-unrefused`), `stops-composed-of-the-crates-own`, and
`writer-showing-a-printing-type-unseen`, now held by the corpus case.

## Amendment: what a `format!` made apart from the line prints (2026-11-12, #1534)

A writer whose calls can't be followed, a `dyn Fn`'s, makes every body
that prints one that may run while a line is formatted, `main` among
them, so `main`'s `format!` is made apart from the line,
`$formatting(() => ..)`, which opens a frame. What a writer printed
inside it was held in the line, as a frame was open, and nothing wrote
it as the frame ended: `main`'s next line, printed at once with no frame
open, overtook it, `format L1` then `<1>stepped 2` where Rust's
`LineWriter` writes `<1>format L1`. Now `$formatting` writes what it
leaves held as its last frame ends, as `$printing` does. Corpus case
format_made_apart_prints_before_the_next_line against native Rust in
both builds; mutation `formatting-held-print-unwritten`.

## Amendment (2026-11-12, #1496): a writer that may panic, where it may be caught, writes through the line

A writer that panicked part-way through a `println!`'s line lost what it
had written: `println!("x {} y", halfway)` caught, then `println!("caught
{}", ..)`, printed `caught true` where native Rust prints `x halfcaught
true`, its stdout's `LineWriter` keeping `x half`. The user decided:
preserve it where it's caught. In a program that calls `catch_unwind`
(`prints.rs`'s `catches_unwinding`), a writer that may stop part-way is
one `prints` lists as it lists one that may print. It writes through
stdout's line (`$sink`), and its `println!` is `$printing`'s, so what it
wrote before its panic stays in the line, as Rust's does. A panic hook,
which could print after a panic too, is refused (`panic::set_hook`). In a
program that catches no panic, it ends on the panic, and the line it had
begun is lost, where Rust writes it as the program ends: ADR 0262's
second case, which the user decided, listed in its Consequences and the
guide's differences (`semantics.md`, 57).

**Which writers may stop, less coarsely.** `never_unwinding` (ADR 0902)
takes any `write!` into a `Formatter` to unwind, so every writer would
have streamed, and every std value's `Debug` of a derived `Debug` been
refused (#1530) in a crate that catches a panic. Now `prints` asks
`unwinding.rs`'s second answer, what never unwinds but through what its
formatting shows (`Unwinding::ButThroughWhatsShown`): std's formatting,
a `Formatter`'s writes and builders, an `Arguments` of any value and a
`Debug` or `Display` call of one, is taken to go on. What it shows and
runs `prints` follows (`stopping`): a body stops where it may unwind so,
returns an `Err` (`failing`), shows a value of a type one of whose
writers stops, or of what can't be known (a type parameter's, a `dyn`'s
read from a place), or runs what does
(`may_run`: a shown value's writer, the iterator a builder's `entries`
steps, a crate's `fmt::Write`), or what can't be followed, to a fixed
point. An exhaustive `match`'s impossible branch, a derived `Debug`'s of
an enum, never runs. Another crate's writer is taken not to stop: it's
shown whole, as a library's that prints is (#1490's). A derived `Debug` of numbers and strings, and a
`Display` that only writes, don't stop: they're shown as before, and a
`Vec` of them isn't refused. The `Formatter`'s own destination, stdout's
line where this asks, never panics; a crate's `fmt::Write` that does is
outside a line. ADR 0902's `never_unwinding` itself is unchanged.

**Only a writer that may write into a line.** What a writer wrote before
its panic matters only in stdout's line: in a `format!`, the `String` is
never given. So of those that may stop, only a writer that may run in a
line streams, and only its type is refused in a composed `Debug`
(`in_a_line`): a type a print shows (each `fmt::rt::Argument` under a
`print!`'s or `println!`'s arguments, `println!("{:?}", v)`'s `Vec<Loud>`'s
`Loud`), and to a fixed point what such a writer may run or shows. All
writers where a print shows, or such a writer shows, what can't be known.
So `debug_entries_drop_as_std`, whose writers that may panic are shown
only in a `format!` under a `catch_unwind`, is as it was; of the whole
corpus only `never_unwinding_calls`' snapshot changes, its `Bad`, shown
by a `println!` that may be caught, now written through the line.
A library's writer that may panic, shown by a consumer's `println!` that
catches it, is shown whole, as one that prints is: no contract fact says
it may (the metamorphic net's `library` rewrite, listed with #1490's).

Proof: corpus case `panic_part_way_through_a_line_kept_where_caught`,
against native Rust in both builds: a writer that panics after writing,
one before writing, one inside another's `write!`, a `Debug`, a
`print!`'s, one that makes a `format!`, and a `to_string()`, each caught;
and `stopping_writers_where_a_panic_is_caught`, both against native
Rust in both builds: a derived `Debug` of
numbers in a `Vec` and an `Option`, `{:#?}` too, and a `Display` that only
writes, shown as before in a crate that catches, and one that may panic
by an index, kept. test/runtime-print.test.ts shows the decided
difference: a line cut short where nothing catches the panic lacks the
partial line native Rust writes at exit.
