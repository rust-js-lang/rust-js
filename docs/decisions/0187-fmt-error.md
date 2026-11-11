# 0187. A writer's `Err(fmt::Error)` is thrown, with what it wrote, and each consumer takes it as std's does

Status: Accepted. Amends [0054](0054-display.md), [0148](0148-write-to-string.md)
and [0166](0166-user-fmt-write.md); extends [0100](0100-separate-crates.md).
Amended: a `fmt::Result`'s `map_err(f)` is `Ok(())`, or `Err` of `f`'s
value, a `Result` of its own, `fmt_result_values`' answer as the others'
are: arrayvec's `ArrayString::try_from` (`test/corpus/fmt_result_map_err.rs`).
(Amended by [0462](0462-io-error-values.md): an `io::Result<()>`'s `Err` is
thrown too, and the helpers that take either are `$writeTry`,
`$writeCheck`, `$writeExpect`, `$writeResult` and `$writeOf`.)

Case: C, D, B ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A writer returns the string it writes (ADR 0054), and a `fmt::Result` is
nothing: writing to a string can't fail, so `Err(fmt::Error)` was an error,
and `is_ok()` was `true` (ADR 0148). chrono's formatting returns one, for
a format it can't write, and stopped there, at four places.

In Rust the error goes up through each `?` to what consumes the text, and
what was written before it stays written. Each consumer takes it as its
own:

| Consumer | What Rust does |
|---|---|
| `write!(s, ..)` to a `String` | gives the `Err` back; `s` holds what was written before it |
| `to_string()` | panics, `a Display implementation returned an error unexpectedly: Error` |
| `format!` | panics, `a formatting trait implementation returned an error when the underlying stream did not: Error` |
| `print!`, `println!` | prints what was written before it, then panics, the same without `: Error` |

## Decision

**`Err(fmt::Error)` is thrown, a `$FmtError`, and a writer it passes out of
puts what it wrote in front of what the error carries. Each consumer catches
it and does what std's does.** `Ok` is `undefined`, as it was.

```js
function spanDisplay_fmt(span) {
  let f = "";
  try {
    f += hoursDisplay_fmt(span[0]);
    f += "-";
    f += hoursDisplay_fmt(span[1]);
  } catch (error) {
    throw $fmtWritten(error, f);
  }
  return f;
}
```

- **A `fmt::Result` as a value is `undefined`, or the error caught:**
  `$fmtTry(() => show(s, h))`; `?` of one passes it on, `is_ok()` is
  `r === undefined`, and `{:?}` is `Ok(())` or `Err(Error)`.
- **A write of what may fail is piece by piece,** `t += "a"; t += x;`, so
  what came before is written; a `write!` to a `String` takes what the
  failing writer wrote, `t += $fmtPartial(error)`.
- **Only what may fail is written so.** A crate's functions that may return
  one are found as its drops are: one that makes one, or calls or formats
  one that may, of the crate's or a library's, which its manifest says. A
  generic `T`'s may where any of the crate's or its libraries' may. A crate
  none of whose do is lowered as before, `is_ok()` `true`.
- **A writer of the crate's that fails is still an error:** it's given each
  `write!`'s text whole (ADR 0166), where Rust gives it a piece at a time,
  so it would fail where Rust's doesn't.

## Why

- **It's exact:** corpus cases compare a `write!` to a `String` that fails,
  its text and its result, a function's `?` of one, and `to_string()`,
  `format!` and `println!` of one, with native Rust; a library's writer
  that fails, given to a consumer's `write!`, too.
- **A crate with no `fmt::Error` is the JS it was:** the corpus's JS is
  unchanged but where a case makes one.

## Alternatives

- **One panic for every consumer:** less code, but `to_string()`'s and
  `println!`'s messages are std's own, and a `write!` to a `String` gives
  the error back, without a panic.
- **A writer writing into its consumer's buffer,** as Rust's does: what it
  wrote would be there without being handed up, but every writer's JS would
  take a buffer, where it returns a string (ADR 0054).

## Costs

- **A library's generic code given the crate's type that fails:** compiled
  without the crate, it doesn't catch the error, which is then `format!`'s
  panic, whichever consumer it was.
- **What may fail is written with `try`s,** a writer's and its consumers',
  where it was a template.

## Amendment: `map(f)` of a `fmt::Result`

**`map(f)` of a `fmt::Result` is `Ok(f(()))`, or its `Err` kept**,
writeable's `write_to(sink).map(Ok)`: its function given `()`,
`undefined`, so a function that takes it as a value, `Ok`'s, is put in
place, `{ TAG: "Ok", _0: { TAG: "Ok" } }`. Where no writer fails, its
answer is constant; else what `$fmtTry` caught is tested, as `map_err`'s
is.

## Amendment: a writer that writes nothing

A writer that fails having written nothing, its `Formatter` unread, `fn
fmt(&self, _: &mut fmt::Formatter) -> fmt::Result { Err(fmt::Error) }`,
read a string it never declared: tidy gives its string `""` where it's
returned, `return "";`, and takes its `let`, and the end this decision
writes, `catch (error) { throw $fmtWritten(error, f); } return f;`, named
it still, a `ReferenceError` where the writer ran. Where nothing in the
body names its string, what it wrote before failing is `""`, `throw
$fmtWritten(error, "")`, and its body gives what it gives, with no `return
f` after. Held by `test/corpus/fmt_error_unwritten.rs`, run as native
Rust, and a mutation in `scripts/mutations/lower/mir/writer.ts`.

## Amendment: a writer's string tidy makes a `const`

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what `write!` gives
and what the string holds after it are Rust's.

A writer that may fail on more than one branch having written nothing,
`if self.0 { return Err(fmt::Error); } if f.alternate() { return
Err(fmt::Error); } Ok(())`, names its string where it returns, and tidy,
seeing it never set again, declares it `const f = "";`. The end this
decision writes found its declaration only as a `let`, so the whole body,
`const` and all, went in the `try`: the `catch`'s `$fmtWritten(error, f)`
and the `return f` after it were out of its scope, a `ReferenceError`
however the writer ended. Its declaration is a `let` or a `const`; either
is kept before the `try`. Found by the nightly's tidy stages (ADR 0659),
whose `fmt_error_unwritten.rs` after `values` declared its string so. Held
by `test/corpus/fmt_error_unwritten_branches.rs`, run as native Rust, and
the mutation `mir-writer-const-string-in-try`.

## Amendment: a write's result made as a value is its error, thrown where a function gives it back (#1011)

"A `fmt::Result` as a value is `undefined`, or the error caught" held of
what `$writeTry` caught, but `Err(fmt::Error)` made anywhere was thrown
where it was made, `$fmtError()`: `let err: fmt::Result =
Err(fmt::Error); println!("{}", err.is_ok())` threw where native prints
`false`, and so did one made into a field, a `RefCell`, an array or a
`Some`. And `is_ok()` was asked only of what `$writeTry` gave or a
variable, so one held in a field or a `Some`, `held.0.is_ok()`, folded to
`true`.

- **`Err(fmt::Error)` made is its error, `new $FmtError()`**, and an
  `io::Result<()>`'s `Err(e)` its `io::Error`, `e`, as what `$writeTry`
  catches is (`adt_value`).
- **Where a function gives one back, `_0 = ..`, it's thrown, as a call of
  it throws** (`returned_write_result`): `Err(..)` made there as it was,
  `$fmtError()` or `$ioThrow(e)`, and one held, `let r = ..; r`,
  `$writeCheck(r)`, which gave the error back as a value before.
- **Any read of one may be its error** where the crate makes one
  (`may_be_write_error`): not `undefined`, nor a call of a function
  `fmt_result_value` didn't catch, which can't fail; a variable, a field,
  an item, a `Some`'s or a cell's.
- **A `write!` to a `String` reached through a call,
  `write!(*s.as_mut().unwrap(), ..)`,** holds the place first and is
  caught piece by piece as one to a variable is; it threw past its
  result.

Results changed ([0262](0262-when-rust-and-js-disagree.md)), so Rust's
are kept; the corpus's JS is otherwise the same. Held by the corpus case
`fmt_error_as_a_value` and the mutations `fmt-error-value-thrown`,
`returned-write-result-unchecked`, `held-write-error-taken-for-ok` and
`string-write-through-a-call-uncaught`; the net's ten rows of #1011 run as
native. A closure that gives back one that may fail is #1042.

## Amendment: a closure that gives back a write's error (#1042)

Which functions may give back a write's error was asked of each body's
maker, `typeck_root_def_id`: a closure that makes one, or passes one on,
`|| -> fmt::Result { made(true)?; Ok(()) }`, marked the function it's
made in, never itself, so its call wasn't caught and its throw went past
what read it, `{:?}` of `checked()` an uncaught `$FmtError` where native
prints `Err(Error)`. And a generic `F: Fn(bool) -> fmt::Result` called,
`f(true)`, gives `<F as FnOnce>::Output`, which wasn't taken for a write's
result, so `run(f)` wasn't found to fail. `fmt_failures` keys each body
by its own id now, a closure's its own, its output its closure signature's
(`output_of`), and normalizes a callee's output as the caller has it. A
closure's call is caught as a function's is (`closure_may_fail`): of the
crate's closure, where `fmt_failures` found it may; of a function, as
`call_may_fail` says; of a `dyn Fn` or a type parameter's, where any
may; and a function a variable holds, `let held = named; held()`, called
as that variable, as its name's call is (`fmt_result_value`), which the
net's `closure-fn` rewrite of the case found. Results changed
([0262](0262-when-rust-and-js-disagree.md)), so Rust's are kept; the
corpus's JS is otherwise the same. Held by the corpus case
`closure_giving_back_a_write_error` and the mutations
`failing-closure-its-makers`, `closure-output-unnormalized`,
`closure-call-of-a-write-error-uncaught` and
`held-function-write-error-uncaught`.


## Amendment: what formatting a value may call is asked of the types it holds, never of a binder's (#1111)

Whether a value's `{:?}` may fail was asked of each type its type holds,
`ty.walk()`'s, `&'a u32` of an `Inspect<_, fn(&u32)>`'s `for<'a> fn(&'a
u32)` too: a type only the `fn`'s binder holds, no value's, of which
rustc's trait selection was asked and crashed ("has escaping bound vars"),
wherever a write of the crate's may fail. The analysis that finds what may
fail skipped those already; the lowering's `fmt_may_fail` didn't, two
answers to one question. `recognition::formatted_parts` is the one now,
asked by both. An audit of each walk of a type that asks trait selection
found no other: `owns_interior_mutability` takes a `fn` pointer for shared
and a `dyn` for not `Freeze` before what their binders hold, and
`rc_counts`' and `std_items`' ask only of a type parameter or a closure.
`test/corpus/debug_of_a_fn_pointers_holder.rs` crashed before, and runs as
native Rust; the neighbour shuffle's seeds 102 and 145, which found it,
run; the mutation `formatted-parts-under-binders` puts the old walk back.

## Amendment: `unwrap_err()` and `expect_err(..)` of a write's result (2026-11-10)

chrono's `write!(buf, "{}", dt.format("%#z")).expect_err(..)` was
refused, `methods of a fmt::Result, from its MIR` (#1160). **`unwrap_err()`
and `expect_err(msg)` of a write's `fmt::Result` or `io::Result<()>` are
`$writeExpectErr(result, msg)`: the error where the write failed, as
`$writeTry` caught it, std's panic where it didn't, `msg: ()`, of
`unwrap_err()` ``called `Result::unwrap_err()` on an `Ok` value: ()``.**
In a crate where no write may fail its result is `Ok`, so it's the panic,
`$writeExpectErr(undefined, msg)`, after what made it ran. Held by
`fmt_result_unwrap_err.rs` and `fmt_result_unwrap_err_never_failing.rs`
against native Rust and five mutations, `fmt-result-unwrap-err-refused`,
`write-expect-err-of-ok-silent`, `write-expect-err-message-unsuffixed`,
`unwrap-err-of-a-failed-write-unread` and `unwrap-err-message-of-unwrap`.

## Amendment: a type parameter's write is caught where it's tested (#1398)

Of every route a call of a write takes, a type parameter's dictionary's,
`W::write_all(out, ..)` called as `WWrite.writeAll(out, ..)`, was the one
that never asked `fmt_result_value`. So in a crate whose writes may fail,
its result tested as a value was taken to succeed:
`out.write_all(b"x").is_ok()` was `true` of a writer that refused it,
`let _ = ..` threw past the function, and `match` of it read the call's
`undefined` while the error went up. **A dictionary's call of a write is
`fmt_result_value`'s too: where it may fail, as any dictionary's may in
such a crate, its value is what `$writeTry` caught, and where it's only
given on, `?` or the function's own result, it stays the plain call,
`fmt_check` unwrapping it.** So a write given on reads `WWrite.writeAll(out,
[120]);`, not `return WWrite.writeAll(out, [120])` or
`$writeCheck(RRead.readExact(r, b))`, and `unwrap()` of one is std's panic
of the error, `$writeExpect`, not the error itself thrown. Found by an
audit of #1392's probes; `test/corpus/generic_write_tested_where_writes_fail.rs`
(an `io::Write`'s and a `fmt::Write`'s, `is_ok()`, `let _ =` and `match`)
runs as native Rust in both builds, and the mutation
`dictionary-write-never-fails` puts the uncaught call back.
