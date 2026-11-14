# 1135. A `write!` into a writer is written a piece at a time

Status: Accepted (2026-11-13, #1652, campaign #1681), landed
(2026-11-14): the one way. Tu's decision (option A, std's exact split);
the lead approved the design. It supersedes [0166](0166-user-fmt-write.md)
and the "one `write_fmt` of its text" rule of
[0180](0180-generic-writers.md).

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
- **What isn't followed is refused, naming #1652**: an `Arguments` made
  elsewhere, one a function returned or a field held, written into a
  writer. A library's type or `dyn` whose contract doesn't say its
  dictionary carries `fmtInto` is refused naming #1688 (step 5).

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
6. An `Arguments` passed on (landed). An `Arguments` stays the function
   of its text. Only where it flows to a parameter that is written into a
   writer is it the function that writes into the sink it claims:

   ```js
   () => {
     const f = $sink();
     f.write("a");
     $writeInteger(f, n);
     return f.done();
   }
   ```

   A write of it arms that sink with its writer's, `$writingInto(f,
   args)`, so each piece is a call of the writer's. Called plainly, by
   `to_string()`, `format!` or `{}`, it gives its text, as `$sink()`
   with nothing armed is a string of its own.

   `analysis::written_params::written_arguments` says which parameters
   are written so, by their MIR, as the lowering reads them, to a fixed
   point through calls. A parameter is written so where it's given:
   - to `write_fmt` into a writer std writes a piece at a time;
   - to `fmt::write`;
   - to a `Formatter`'s `write_fmt`;
   - or to another function's parameter written so.

   A trait's method of the crate's is marked where any of its impls is,
   for calls through a `dyn`. A library's contract says it,
   `Calling.written_arguments` (summary, library ABI 36). A consumer
   compiles a library's impl methods itself, so it finds those by the
   same analysis.

   A `format_args!` given such a parameter shows its types into a writer,
   so their writers are framed. A function given an `Arguments` made
   elsewhere, one a function returned or a field held, is refused, naming
   #1652.

   Rust's `write!` calls `W::write_fmt`, so a write through a dictionary,
   a `W: fmt::Write`'s or a `dyn`'s, is `W.writeFmt(w, args)`, given that
   function. Each dictionary's `writeFmt` takes it:
   - std's, of a writer without its own `write_fmt`, is
     `$writerWriteFmt(writeStr, writeChar)`, which writes it into
     `$writerSink`;
   - a writer's own `write_fmt` is called with it as it is;
   - `$mutWriter` and a `dyn` pass it on;
   - `$stringWriter` appends its text, keeping what was written before a
     value that fails (#1696);
   - a `Formatter` given as a `W`, a framed writer's sink, has it written
     into that sink, and its `write_char` is the sink's.

   Before this, under the switch, generic code wrote each piece into
   `W.writeStr`, past a writer's own `write_fmt`, which Rust calls.

   A concrete writer of the crate's own without its own `write_fmt` keeps
   the inline form. `fmt::write` is std's own `write_fmt`, through the
   writer's `write_str`, never its own `write_fmt` (#1697, ADR 0446,
   amended).

   Proof, each against native Rust in both builds:
   - test/writer-sink-arguments.rs: parameters, a relay, `dyn` and own
     trait methods, `fmt::write`, a `Formatter`'s `write_fmt`, failures,
     and the same `Arguments` written and printed;
   - test/writer-sink-own-write-fmt.rs: generic code given a writer with
     its own `write_fmt`, `&mut W`, an `Either`, a `Formatter` as `W`;
   - a crate pair: a library's functions and writers;
   - the formatting grid: each pieced case's `args` is also given to
     `logged`, which prints it, the sink form called plainly, and writes
     it into a `Log`, 1,370 cases, both the same as native Rust's.

   Also the metamorphic rewrites of each case, and 26 mutations.
7. The rest (landed):
   - A map's and a set's `Debug`, by `debug_map`'s and `debug_set`'s
     `entries`, each in the order its `Debug` shows them: `$writeDebugMap`
     and `$writeDebugSet` of the runtime's one builder model.
   - A derived `Debug` of more than five fields,
     `debug_struct_fields_finish` and `debug_tuple_fields_finish`, written
     as one of fewer is.
   - An integer's `{:e}` and `{:E}`, as core's `exp_u64` writes it,
     `$writeIntExp`: its coefficient's trailing zeros dropped, or rounded
     half to even to the precision asked. With no zeros to add, it is
     `pad_integral` of its text; else `pad_formatted_parts` of the
     coefficient, the zeros and the exponent (`$writeFormattedParts`,
     a float's too).
   - A writer reached through a value given its `Formatter`, a closure, a
     `fn` pointer or a `dyn Fn`, is framed as the writer that calls it is.
     It is called with that writer's sink armed, `$writingInto(f, w, n)`:
     - a closure the writer calls, by its type;
     - and, where it calls one only a value says, every closure of the
       crate's given a `Formatter` and every writer function named as a
       `fn` pointer.

     A closure lowered where its maker is, a boxed one's, is the writer
     its own body is, `writer_owner`.
   - A `Formatter` a struct holds, `held.0`, which a writer that writes
     through a sink holds its sink as. A `write!` into it is written into
     that sink a piece at a time, as one into the writer's own is
     (`written_into_a_formatter`, one rule for both). A `write_char` is
     the sink's, and an `Arguments` parameter is `$writingInto(held,
     args)`.
   - A `dyn Debug`'s `Debug`, by its pair's dictionary, its type's `Debug`,
     `d.impl.fmtInto(f, d.value, options)`, a builder's step's too. A
     library that makes one says so, `makes_fmt_dyns`, and a consumer
     writing it is refused naming #1688.

   Proof, against native Rust in both builds:
   - test/writer-sink-rest.rs;
   - test/writer-sink-callables.rs, a case of its own, as a `dyn` written
     frames every writer;
   - the `dyn Debug` rows of test/writer-sink-generic.rs and the closure of
     test/writer-sink-own-write-fmt.rs;
   - the formatting grid, whose `Log` column now pieces every value but
     `Asked`'s, a map's, a set's and an integer's `{:e}` among them;
   - a library pair refused;
   - 15 mutations.
8. What the switch-over's first full runs found (landed, under the
   switch). The corpus and the crate corpus with the switch on refused or
   crashed where they compile today:
   - **A library publishes its formatted types** (the lead's decision,
     option A). A consumer may write any type reachable from outside a
     library into a writer of its own, `write!(log, "{:?}",
     dep::Hexy(255))`, so:
     - the library frames each formatting impl of such a type of no type
       or constant parameters (`Prints::published`), one sink body that
       serves both a plain call, which gives its text, and its
       dictionary's `fmtInto`;
     - its contract lists the type (`written_into`);
     - a consumer writes it by that dictionary, `Dep.hexyDebug.fmtInto(f,
       h, options)` (`Foreign::lists_fmt_into`);
     - private types keep their form. A generic one isn't published yet
       (#1712).

     Measured with the switch on: the React port's regeneration
     unchanged; the readability gate unmoved; the 7 crates' verdicts as
     before.
   - A framed writer writes every formatting trait of the crate's own
     type, `{:x}` of its own `LowerHex` too. A `{:x}`, `{:e}` or the like
     of a std number given the `Formatter`, `LowerHex::fmt(&self.0, f)`, is
     written by the `Formatter`'s options as it runs, an integer's `{:e}`
     too.
   - A `Cell`'s and a `RefCell`'s `Debug` is `debug_struct`'s one field,
     and `<borrowed>` of a `RefCell` mutably borrowed (`$writeBorrowed`).
   - Two ICEs are gone. Framing asked `fn_sig` of a constant's body, and
     now asks only function bodies. The analyses classified a body's calls
     in the crate's empty typing environment, and arrayvec's
     `ArrayString<CAP>` crashed; each body's calls are now asked within
     its own (`written_params::within`).
   - #1710 is fixed, in every program: a generic writer function's
     instance as a `fn` pointer takes no `Formatter`, as its JS doesn't.
     `written_params` sees an instance named so as a call.

   Proof:
   - test/writer-sink-rest.rs (cells, own and std radixes and exponents);
   - test/writer-sink-const-generic.rs and test/writer-sink-fn-pointer.rs;
   - a published library pair;
   - the corpus case writer_fn_pointer_at_an_instance;
   - 13 mutations.

   Then, each its own commit:
   - **An `Arguments` kept is the function that writes into the sink it
     claims** (the lead approved, under the switch). Step 6 made one sink
     form only where a write was followed to; one in a field, an
     `Option`, a closure's capture, a value returned, a constant, or given
     to a generic function, `Box::new(args)`, was plain, and a write of it
     was refused, "made elsewhere". `given_to_a_write` now counts each of
     those as kept, and a read of a place or of a call's result is sink
     form (`sink_form_given`). So `Display::fmt(&args, f)`, strum's
     derived `Display`, `Display::fmt(&self.0, f)` and
     `f.write_fmt(given(..))` write it a piece at a time, as std's
     `fmt::write` does. A sink-form function called plainly gives its
     text, so one kept only for plain text is still right; at the flip
     the count of those made sink form is measured, and one whose only use
     is plain text kept a template. A `&Arguments` parameter is followed
     too. `Display::fmt(&format_args!(..), f)` arms an `Arguments`
     function where it could write inline (#1725, P3). Proof:
     test/writer-sink-arguments-kept.rs; 8 mutations.
   - A std parse error's `Display` given the `Formatter`, thiserror's
     `Display::fmt(&self.0, f)` of a `ParseIntError`, is `pad` of its
     message, as a `dyn Error` of it writes it. Proof:
     test/writer-sink-rest.rs; 1 mutation.
   - A `Layout`'s `Debug`, smallvec's derived `Debug` of a `&Layout`, is
     written a piece at a time as its derived `Debug` writes it: its
     `size` a `usize`'s by the options (#1728), its `align` an
     `Alignment`'s `write!` of its own (`$writeDebugLayout`). Proof:
     test/writer-sink-rest.rs; 2 mutations.
   - A parse error's `Debug` in a framed writer, thiserror's derived
     `Debug` of a `&ParseIntError`, is its derived `Debug`'s pieces, its
     kind one `write_str` (`$writeDebugParseError`). Proof:
     test/writer-sink-rest.rs; 2 mutations.
   - A library's derived `Debug` of more than five fields makes `&dyn
     Debug`s of its fields that only std's `debug_struct_fields_finish` is
     given, which never leave it: it no longer counts as making a `dyn` a
     consumer may write (`makes_fmt_dyns`), so a consumer's `dyn` written
     into a writer beside it isn't refused (#1688's second case, chrono's).
     Proof: a piecewise pair; 1 mutation.
   - A published type whose formatting impl the library exports none
     of, a derived `Debug`, which a consumer compiles its own of (ADR
     0914, amended): the consumer frames its copy, and its dictionary
     carries `fmtInto`, as the library's would (`listed_writers_compiled_here`).
     A published type of a lifetime alone, `Prefixed<'a>`, is published as
     one of none. Proof: the published piecewise pair; 2 mutations.
   - An `impl Display` a writer gives its `Formatter`, anstyle's
     `self.effects.render().fmt(f)`, is the type it hides where framing
     reads what a writer shows, as its MIR is lowered, so that type's
     writer is framed. Proof: test/writer-sink-impl-display.rs; 1 mutation.
   - A type reached only as a field of the crate's type, whose writer
     never shows it, rustls-pki-types' private `BytesInner` in `Der`, which
     framing leaves, carries no `fmtInto` and isn't refused: nothing writes
     it so (`WrittenParams::given_adt`, the types given a parameter written
     so or held by one). Proof: the published piecewise pair; 1 mutation.

9. The switch-over (landed). `writes_piecewise()` and
   `RUST_JS_WRITES_PIECEWISE` are gone, with the default mode's whole-text
   paths: a writer's own `write_fmt` entry that wrapped generic code's
   text as a function, the `$fmtTextOr` call of a failing generic write
   (#1696's interim patch), `$stringWriter`'s branch of a text, and the
   formatting table's environment. test/writer-sink*.rs are corpus cases,
   `writer_sink_*`, against native in both builds and as a library.
   Library ABI 37.

   **An `Arguments` is the function that writes into a sink only where
   something may arm one** (`WrittenArguments::arms`, `arms_sinks`): a
   library; a writer shown into a writer, `framed`; or a call that writes
   one it's given into a writer a piece at a time, a writer of the crate's
   own or a generic one, `fmt::write`, a function that does. A
   `Formatter`'s `write_fmt`, or a writer function's given its
   `Formatter`, of a writer not framed, writes its text. And it's kept by a
   generic function only where its type parameter is in what that gives
   back or in another parameter, `Box::new(args)`, `v.push(args)`; one that
   only reads it, `args.to_string()`, makes its text at once. So one whose
   only use is its text stays its template (the lead's ask). Measured on
   the corpus: 14 `Arguments` became the function that writes into a sink
   where they were a template, each written into a writer of the crate's
   own or a generic one, `make(8, () => { const f = $sink(); f.write("1-2");
   .. })` given to a writer's `write_fmt`, and 6 templates went; before the
   gate and the narrowing, 40 and 33. The React port: none
   (`react-in-rust.ts regen`'s diff is other commits').

   The switch-over's last run found, each fixed here:
   - A std formatting trait's `fmt` taken as a `fn` pointer,
     `self.format(f, fmt::Display::fmt)`, was its text where a framed
     writer armed it: "didn't write through its sink" at run time, in a
     program that runs today. It's now the function that writes into the
     sink it claims, by the `Formatter`'s options as it runs, where a
     writer is framed (`std_fmt_fn_value`), its text called plainly.
     Corpus case writer_sink_fn_value_of_a_std_fmt.
   - A library's writer function given a consumer's framed `Formatter`,
     the library's or the consumer's own copy of it, is framed
     (`Foreign::frames_writer_fn`, `library_writer_framed`), and the
     consumer arms it. A writer armed whose callee doesn't claim the sink
     lets a callee of that callee claim it instead, a wrong answer, so
     framing is held the same on both sides. Pair
     librarys_writer_function_given_a_consumers_formatter, and
     test/crates/pairs/library_writers_given_options.
   - A library publishes its formatted types, which a program alone
     doesn't: test/crates-compiled-here's library and program forms differ
     by them now, and two tests' library JS reads so.
   - The metamorphic `library` rewrite of the moved cases found two more,
     which only the switch-over shows: a library's trait's method that
     writes the `Arguments` it's given, called through a `dyn`, wasn't
     known to by its consumer, as its contract lists items alone, so the
     consumer gave a template and a writer it showed claimed the sink, its
     own pieces lost (#1735, a wrong answer); its contract now lists its
     traits' methods' too (`written_trait_arguments`). And a consumer's
     callable given the `Formatter`, called by a library's writer, wasn't
     framed (#1736); it is wherever a library it uses writes a piece at a
     time. `$writingInto` now refuses a writer that gives back its text, a
     writer it called having claimed the sink in its place: an error, never
     that text. Pairs arguments_through_a_librarys_dyn_trait and
     consumers_closure_called_by_a_librarys_writer.

10. After the switch-over (amended 2026-11-14, #1738, #1739), two throws
    in programs that ran before it, each a generic type of the crate's
    own written into a writer, so its writer is framed:
    - **Each instance of a generic formatting impl the crate asks for gives
      its type parameters dictionaries that carry `fmtInto`** (#1738). A
      framed writer writes its parameters by its dictionaries,
      `TDisplay.fmtInto(f, x)`, however it's called, so
      `println!("{}", Wrap(String::from("s")))` beside a `write!` of
      `Wrap(3)` into a writer threw, `String`'s dictionary carrying none.
      `written_params::fmt_impls_asked` lists each instance the crate's
      bodies ask for: each part of each type a body gives a function,
      `Argument::new_display::<Wrap<String>>`, `to_string`'s, a generic
      caller's bound's, or makes a `dyn` of, of the crate's own generic
      types and of a library's its contract lists, is asked of each
      formatting trait it has an impl of, by trait selection within the
      body. `written_params` gives what each instance's arguments are at
      the parameters its impl's writer writes, to its fixed point, so a
      generic caller's own parameter, `format!("{}", Wrap(t))`'s `T`, is
      written so too. And framing frames what each instance of a framed
      writer gives it, as a type shown: `Wrap(Point(1, 2))` formatted
      plainly frames `Point`'s writer, whose dictionary's `fmtInto` arms
      it. A dictionary of a type put in a framed generic type may carry
      `fmtInto` where nothing formats it: the answer is by type, as
      `reach`'s already was (the lead accepted the cost). Measured: no
      other corpus case's dictionary gains one, nor the React port's.
    - **A generic impl's dictionary's `fmtInto` gives its writer its
      options only where it takes them** (#1739), as `writer_given` calls
      it, asking `takes_options`: `$writingInto(f, wrapDisplay_fmt, value,
      TDisplay)`. It gave them always, so where the crate's writers take
      none they landed where its dictionaries belong, and a
      `Wrap<Wrap<f64>>` written into a writer threw. A non-generic impl's
      `fmtInto` there reads `(f, value) => $writingInto(f,
      pointDisplay_fmt, value)`, 7 corpus snapshots.

    Proof, each against native Rust in both builds and by its metamorphic
    rewrites: the corpus cases writer_sink_generic_instances and
    writer_sink_generic_instances_with_options, the second's writers
    taking options; 6 mutations.

    And a third (#1743): **a library's type it gives out as an `impl
    Trait` is published**, as one reachable from outside it is, through a
    private function's `impl Trait` too: `pub fn hidden() -> impl Display {
    Hidden(3) }`, its `Hidden` written through a consumer's generic writer,
    threw, as `effective_visibilities` doesn't call `Hidden` reachable, so
    its writer wasn't framed and its dictionary carried no `fmtInto`; and
    `refuse_lacked` didn't see through the `impl Display` to refuse it.
    Each reachable function's `impl Trait`, revealed, publishes each type
    of the crate's it hides, and a consumer refusing a library's type its
    contract doesn't list asks of the type an `impl Trait` hides. Proof:
    the pair librarys_hidden_type_through_generic_code against native, and
    of a contract that doesn't list them, refused; 3 mutations.

## Alternatives

- **Keep ADR 0166's whole text**, listed as a difference: rejected, since
  the results change (Tu, 2026-11-13).
- **Switch over at the first step, refusing what isn't followed yet**:
  rejected, because either, arrayvec, itertools, rust_decimal, anyhow, toml
  and toml_edit would go from compiling to refused.
- **One long branch**: rejected, since rebasing it onto a main that moves
  by a hundred commits a day costs more than landing each proven step.
