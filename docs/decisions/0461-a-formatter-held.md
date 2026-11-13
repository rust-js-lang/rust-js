# 0461. A `Formatter` held is a handle on its writer's text, and a `fmt::Result` meets generic code as a `Result`

Status: Accepted. Extends [0054](0054-display.md),
[0099](0099-mut-references.md), [0180](0180-generic-writers.md) and
[0187](0187-fmt-error.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)): the same text, and
the same `Err` where it's made.

## Context

base64's `Display` hands its encoder the `Formatter` in a struct of its
own, which the encoder's generic code writes to through a trait whose
error is the impl's:

```rust
impl Display for Base64Display<'_, '_, E> {
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), fmt::Error> {
        let mut sink = FormatterSink { f: formatter };
        self.chunked_encoder.encode(self.bytes, &mut sink)
    }
}
```

A writer's `Formatter` was its text, a local string (ADR 0054), and any
other use of it was refused. Through generic code, a `fmt::Result`,
`undefined` or its error thrown (ADR 0187), met a `Result<(), S::Error>`,
`{ TAG }`: the generic code read `undefined.TAG` and failed, as a `String`
sink whose `Error` is `fmt::Error` did already. And a writer's last call,
`check(n)` of a `fmt::Result` that may fail, gave `Ok`.

## Decision

**A `&mut Formatter` held anywhere but its writer is a handle on the
writer's text, as a `&mut String` is (ADR 0099), and its writes are a
`String`'s through it.**

```js
function showDisplay_fmt(show) {
  let f = "";
  const sink = { f: { get value() { return f; }, set value(value) { f = value; } } };
  ..
}

function formatterSinkSink_write_encoded(formatterSink, encoded) {
  formatterSink.f.value += encoded;
}
```

1. **A `Formatter` is its text, a string**, `is_std_wrapper`'s, as a
   `String` is: a `&mut` to one is a cell.
2. **A struct that holds the writer's `Formatter`**, `FormatterSink { f
   }`, holds a handle on its text.
3. **`f.write_str(s)`, `write!(f, ..)` and `f.write_char(c)` of a held
   one** are a `String`'s, `formatter_write`: `f.value += s`.

**A `fmt::Result` is a `Result` where generic code has one, and the
`fmt::Result` it is where it's read again, `is_generic_fmt_result`.**

- **An impl's, given through a dictionary whose trait method gives
  `Result<(), Self::Error>`**, is `$writeResult(() => ..)`: `{ TAG: "Ok"
  }`, or `Err` of the error thrown.
- **A generic function's `Result<.., S::Error>` read where it's a
  `fmt::Result`**, `generic_result_values`'s, is `$writeOf(..)`: `undefined`,
  or its error.
- **A call of one may fail where any of the crate's may**: its
  dictionary's calls give its error, so the analysis of what fails takes
  it as a dictionary's.

### Also here

**A writer's last call, its `fmt::Result` returned, is checked**, as
`?` of it is: `check(n)` throws where it fails, which the writer passes
on, `$writeCheck($writeOf(encode(..)))` of a generic one. It gave `Ok`.

## Why

- **It's the representation each already has**: a `&mut` to a string a
  handle on it, a `fmt::Result` in concrete code and a `Result` in
  generic code. Each boundary converts, where `generic_result_values` and
  the dictionary's entry already convert a generic `&mut T` (ADR 0454).
- **It's exact**: `test/corpus/formatter_sink.rs` writes through a
  `FormatterSink` and a `String` sink of `fmt::Error`, fails in each, and
  fails in a writer's last call, against native Rust; a mutation of each
  part is caught.

## Consequences

- A held `Formatter`'s options, `f.width()` or `f.pad(s)` through the
  struct, aren't the handle's: each is refused.

## Amendment: every write's result generic code holds, and std's iterators (#1408, #1410)

The rule above held of `Result<(), E>` of a `fmt::Error` alone, given back.
Given to generic code, `report<T: Debug>(r: io::Result<T>)` called as
`report(writeln!(v, "x"))`, a write's result kept its own form and the
callee read `undefined.TAG`; given back from `passed<T>(r: io::Result<T>)`,
`{ TAG }` was read as a write's; an `io::Result<()>`'s had no rule at all.
**A write's result meets generic code as a `Form` of ADR 1115's, decided by
`form_differs` in `generic_facts.rs` alone:**

- **`Form::Tagged`**: a `Result` of generic code's type parameters,
  `io::Result<T>` or `Result<(), E>`, whose caller's is a write's result.
  Given, `$writeTagged(r)`; given back, `$writeOf(r)`; a closure's result
  generic code takes, `$writeResult(() => f(..))`. A trait's generic
  method's, a generic method's, a `fn` pointer's and a closure's
  `F: Fn() -> io::Result<T>` cross so, as a call's do; one held inside,
  `Vec<Result<T, E>>`'s items or a `&mut` of one, is refused, naming
  #1416.
- **`Form::Valued`**: a type parameter, `B` of std's `map<B, F>` or `T` of
  `same<T>(t: T) -> T`, whose caller's is a write's result: held as a
  value, `undefined` or its error, where a call of the caller's function
  throws its error. A closure given there gives it back
  `$writeTry(() => f(..))`, as an item; a generic function of the crate's
  or a library's gives it back to its caller `$writeCheck(r)`, and its call
  may fail where any write may; a trait's method, `from_residual`'s `Self`,
  is its impl's, of the caller's own form.

std's iterators are generic code too (the user's decision, 2026-11-11:
convert at the boundary, where the boundary is). A callback that gives a
write's result gives it as a value, `$writeTry`, in the plain build, as the
`Valued` form is in the build of std's structs; a collect into a `Result`,
which reads each item as one, is given each tagged, `std_reads_tagged`,
and a write's result it collects into given back, `$writeOf`. Concrete code
between keeps the write's form: `filter(|r| r.is_ok())` reads
`r === undefined`. `flatten()` and `IntoIterator` of a write's result take
its `Ok` by the one test of it, `variant_test`; `try_for_each` into an
`io::Result<()>` is `forEach`, as into a `fmt::Result`; and a call of
std's iterator code that gives a write's result may fail where any write
may. Corpus cases `write_result_given_to_generic_code` and
`write_result_items` run as native Rust in both builds; the latter is one
of `test/iterator-forms.test.ts`' few.

## Amendment: a `fmt` trait's method called on a held `Formatter`

`Display::fmt(&x, self.0)`, `Debug::fmt` or another `fmt` trait's,
called as a function on a `Formatter` held in a struct (2026-10-21), as
anyhow's `Quoted` writer escapes what it's given into the one it holds:
what the method writes, appended to the handle's text, `self.0.value +=
..`, as `self.0.write_str(s)` is. Plain, as the handle keeps no options
(above). `recognition::formatter_fmt_to` names it, `Std::FmtTo`;
`std_calls.rs` writes it beside `push_str`. The corpus's
`fmt_to_held_formatter.rs` holds anyhow's `Quoted` and a generic `show`
through a held `Formatter` against native Rust.

## Amendment: a call's `Form::Valued` crossing has one owner (2026-11-13, #1634)

A trait's method whose signature gives a type parameter back, `fn is(self)
-> T`, called through a dictionary with `T` a write's result, threw the
write's `Err` where Rust gives it as a value: `const again =
$writeCheck($writeTry(() => IsResult.is(w)))`. Two places crossed a call's
`Form::Valued`. `generic_result_values` (mut_refs.rs) threw it,
`$writeCheck(r)`, and `fmt_result_value` (display.rs) caught a call's
throw, `$writeTry(..)`. A generic function's call did them in that order
(`mir/calls.rs`). A trait method's call did `fmt_result_value` inside
`trait_call`, and `generic_result_values` after it (`mir/dispatch.rs`), so
the throw landed outside the catch.

**`fmt_result_value` is now the one owner of a call's write result:** of a
`Form::Valued` crossing it makes `$writeTry(() => $writeCheck(r))`, the
error thrown as a call of the caller's throws it, then caught with what the
call threw. A dictionary's entry that holds it as a value, `impl<T> Is<T>
for T`'s, and one that throws it, `impl Is<fmt::Result> for Writes`'s, give
the same, and `?` of it unwraps to `$writeCheck(IsResult.is(w))`, which
throws either way. `generic_result_values` leaves `Form::Valued` alone, and
`form_back` of it is unreachable. Its output is asked of the normalized
type, so `made<M: Make>(m) -> M::Out` with `M::Out` an `io::Result<()>`
crosses too, where its call threw the `Err` past the `let`.

Results change, so Rust's behavior is kept (ADR 0262). Found by the
metamorphic net's `impl-owned` rewrite of `printing_std_wrappers_stopped_part_way`.
Corpus case `write_result_through_a_dictionary` against native Rust in
both builds.

## Amendment: a writer that writes through a sink lends the sink (2026-11-13, #1637)

A writer that may print as it formats writes its text through a sink,
`const f = $sink()` (ADR 1130). It lent its `Formatter` as any writer
did: a cell over the variable, `[{ get value() { return f; }, set
value(value) { f = value; } }]`, or to generic `fmt::Write` code a box
taken back, `const w = { value: f }; ..; f = w.value`. A held write,
`quoted[0].value += s`, then made `f` a string, and the writer's next
`f.write(..)` threw `TypeError: f.write is not a function`. So anyhow's
`Quoted` whose drop prints, and base64's `FormatterSink` whose drop
prints, crashed.

**The sink is itself a handle on its text, as a `String`'s `&mut` is,
and a writer that writes through one lends it as it is:**

```js
function messageDebug_fmt(message) {
  const f = $sink();
  f.write('"');
  const quoted = [f];
  quotedWrite_writeStr(quoted, message[0]); // quoted[0].value += ..
  ..
}
```

A sink's `value` set writes what it's given, and a read of it gives `""`,
so an append, `h.value += s`, writes `s` and nothing else. That read is
safe only because Rust can't read a `Formatter`'s text: neither a held
`&mut Formatter` nor generic `W: fmt::Write` code has an API that reads
what was written, so the one read JS makes of the handle is an append's.
A `String`'s `&mut` is a different handle, a cell over the string, which
reads its text: `held_formatter_prints` holds a `&mut String` in the same
writer and reads its length, so the two never merge. A held write
keeps its one form, `h.value += s`, whichever writer lent the handle, and
generic code's `$stringWriter` takes the sink as it takes a `String`'s
cell. A writer's own writes stay `f.write(..)`. `writer_set_up` lends the
sink where `writer_owner_writes_through_a_sink` says the writer's text is
one, the same fact that makes it (`writer_text_made`), and a closure that
captures the sink takes it as it is, as a `&mut` to a variable is taken.

The alternative was a crate-wide fact, that held `Formatter`s are sinks,
with `.write(..)` at each held write. It was rejected: every writer that
lends its `Formatter` would have had to become a sink, and generic code a
second dictionary.

The result changes, so Rust's behavior is kept (ADR 0262). Corpus case
`held_formatter_prints` runs against native Rust in both builds:
`Quoted` whose drop prints, a `FormatterSink` that generic code writes
whose drop prints, and a writer that prints and gives its `Formatter` to
generic code and to a struct. Found by the metamorphic net's `loud`
rewrite of `fmt_to_held_formatter` and `formatter_sink`.
