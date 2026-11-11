# 0060. `{:?}` by the type, and a derived `Debug` is a function

Status: Accepted. Extends [0054](0054-display.md).
(Amended: a `Debug` another crate's derive writes is the crate's own impl,
ADR 0186.)

Case: C, D, B ([0262](0262-when-rust-and-js-disagree.md)).

## Context

`{:?}` was `$debug(value)`, a runtime helper that looks at the JS value
alone. So it couldn't show what Rust shows:

| Rust | `$debug` |
|---|---|
| `Point { x: 1.0, y: 2.0 }` | `{ x: 1, y: 2 }`: no name, and `1.0` is `1` |
| `(1, "a")` | `[1, "a"]`: a tuple is an array |
| `None`, `Some(3)` | `()`, `3` |
| `'c'`, `Dot` | `"c"`, `"Dot"` |

`assert_eq!`'s failure message showed its values the same way. The type
that would tell them apart is known when compiling, and nowhere else.

## Decision

**`{:?}` is written from the value's type, and a `Debug` impl is a
function, derived or not.**

- **Std's types are shown in place:**
  - numbers are `String(n)`, and an `f64` is `$debugF64(x)`;
  - strings are `$debugStr(s)`, and a `char` is `$debugStr(c, "'")`, `'c'`,
    escaped as Rust escapes them (ADR 0063);
  - an `Option` is `Some(..)` or `None`, a tuple `(a, b)` (and `(a,)`), a
    `Vec` `[..]`, a map `{k: v}`, a `Result` `Ok(..)` or `Err(..)`, and an
    `Ordering` `Less`, `Equal` or `Greater`.
- **A `#[derive(Debug)]` is lowered from the body rustc derives,** like a
  hand-written `fmt` (ADR 0054):

  ```js
  function pointDebug_fmt(point) {
    return "Point { x: " + $debugF64(point.x) + ", y: " + $debugF64(point.y) + " }";
  }
  ```

  - That body hands each field to `Formatter::debug_struct_field2_finish`
    and the like as a `&dyn Debug`. **A `&dyn Debug` is the string it
    shows,** since showing is all one is for, so those calls join names
    and strings.
  - A struct with more than five fields gets arrays of names and strings,
    joined by `$debugFields`.
- **A derived `Debug` is left out unless something shows the type,** or
  shows one that shows it. `#[derive(Debug)]` on every type costs nothing
  in the JS.
- **Generic code:** `T: Debug` takes a dictionary, `{ fmt }`, and it's
  `TDebug.fmt(x)`.
- **`assert_eq!` and `assert_ne!` show their values the same way,** as
  strings made where the assertion fails.
- **Still errors:** `{:#?}` (with line breaks), and `{:?}` of a std type
  rust-js doesn't model. A `Result`'s `unwrap()` still shows its error
  with `$debug`.

## Why

- **It's Rust's output, character for character.** Differential tests
  check structs, tuple structs, unit structs, enums of all three kinds,
  six-field structs, generics, `Option`s, tuples, `Vec`s, `Result`s,
  `char`s with quotes, and `Ordering`.
- **Derived impls read as a person would write them:** one function
  returning a string, with a `return` per variant.
- **No cost for what isn't shown.** Only the `Debug` impls something
  reaches are in the output.

## Alternatives

- **Type descriptors for `$debug`** (`$debug(value, shape)`). A runtime
  walk of a description of the type. It would handle recursive types as
  well, but every `{:?}` would carry a description, and the derived impls
  would be data rather than code.
- **Type names in the values** (`{ $type: "Point", .. }`). It would let
  `$debug` read the name, but it changes every object for the sake of
  debugging.

## Consequences

- A `Debug` impl's function can be called from JS: `pointDebug_fmt(p)`.
- The two-pass lowering (ADR 0049) also decides which derived `Debug`
  impls are used, from the first pass's references.
- A fieldless enum's derived `{:?}` is the value itself, its variant's
  name (ADR 0013), unless `rust_js::name` gave a variant another: then a
  table of them gives the Rust one, `{ "best fit": "BestFit", lookup:
  "Lookup" }[fit]`, of a tagged enum's tag (ADR 0284). (Amended: it was
  the JS name.)

## Amendment: a `Box<dyn Debug>`

A `dyn Debug` is the string it shows, made where a `&dyn Debug` is; but
`is_dyn_debug` saw through references alone, so a `Box<dyn Debug>` or an
`Rc<dyn Debug>` was its value, interpolated raw: `0.30000001192092896`,
`[object Object]` (#174). It sees through a `Box` and an `Rc` that isn't
counted, each its value (ADR 0023); a counted one, a library's, whose
value would be the string where its count is kept, is refused. Held by
`test/corpus/boxed_dyn_debug.rs`.

## Amendment: a `Weak`'s

`{:?}` of an `rc::Weak` or a `sync::Weak`, a field's too, was refused. It's
`(Weak)`, as std's shows one, of none of what it points at. Held by
`test/corpus/weak_debug.rs`.

## Amendment: a pair upcast where it's made by a call

`show(&*shapes[1])`, a `&dyn Shape` of `Shape: Debug` upcast to the `&dyn
Debug` it shows, was refused: "showing a `dyn Shape` made where it's
shown". The pair is read twice, for its dictionary and its value, and
`shapes[1]` is a call. It's held once where it's coerced, `const shown =
..`, which the lowering there can write. Held by
`test/corpus/dyn_shapes_shared.rs` and the mutation
`dyn-debug-pair-unheld`.

## Amendment: a value whose text doesn't read it is still made (2026-11-09, #1063)

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's are kept. `{:?}` of a `()` is `"()"`, of a `Weak` `"(Weak)"`, and
`{}` of a `FromUtf16Error` its message, none reading the value; the value,
`give()`'s or `decoded(&units).unwrap_err()`'s, was dropped with it, so the
call never printed, nor panicked on an `Ok`. **`debug_string_with` and
`display_string_with`, which every `{}` and `{:?}` goes through, make the
value first where it has effects and the text doesn't read it**
(`evaluated_first`), and a `()` given its `Formatter`'s options does the
same: `console.log(`${(give(), "()")}`)`, a comma sequence where the value
is shown, the user's choice of 2026-11-09 over a statement before it (a
rule lifting a sequence's leading items out of a statement, filed as a P3).
One snapshot changes, a value Rust makes that JS dropped: `text_edits.js`'s
`$unwrapErr($fromUtf16([56606]))`. A union's `()` field, which `unions.rs`
shows, reads as `undefined` with no `$unionField`, which refused it where
the union holds another field: a field of no bytes reads nothing, whatever
the union holds (ADR 0406), so its `{:?}` is `"()"` as before.
Held by the corpus case `text_of_a_value_made_by_a_call` and the mutations
`shown-value-unevaluated`, `debug-value-unevaluated`,
`display-value-unevaluated`, `formatted-unit-value-dropped` and
`unit-union-field-read-whole`.

## Amendment: a `dyn Debug` stored or passed is a pair (#1335)

A `dyn Debug` was the string it shows, made where the `&dyn Debug` was
made. That ran its `fmt` at the wrong time, which is a wrong answer whenever
`fmt` does something (#1378, P0, found by audit):
`let d: &dyn Debug = &loud; println!("made"); println!("{d:?}");` ran
`fmt` before "made". `format!("{d:?} {d:?}")` of one passed in ran it once
and showed the same text twice. A function given one that never shows it
ran `fmt` anyway, and so did a `Box<dyn Debug>` where it was boxed.

The user decided (2026-11-11) that **a `dyn Debug` that's stored or passed
is the pair `{ value, impl }`**, as a `dyn Read` is (ADR 0425): `impl` is its
type's `Debug` dictionary, shown as `d.impl.fmt(d.value)`, or as
`d.impl.fmt(d.value, { alternate: true })` for `{:#?}`. `Debug` is one of
std's pair traits (`is_std_pair_trait`), as `Display` is. **One shown right
where it's made stays the string**: a placeholder's `&x as &dyn Debug`, a
builder's step, and a derived `Debug`'s fields, each made a `&dyn Debug`
and given straight to the `Formatter`. A coercion to `dyn Debug` is a
`Value::DynDebug`, kept until it's read. Where it's shown in place
(`shown_in_place`, a placeholder's `Argument`, a builder's step), it's the
string it shows, as pretty as its writer. Anywhere else it's made the pair
(`dyn_debug_pair`). A temporary of one, which a derived `Debug` holds while
its next field is made, is kept too (`flush`), and what it's made of is held
there only where making it does something; borrowck keeps what it borrows
unchanged till it's shown. A `Box<dyn Debug>` made a `&dyn Debug` is the same
pair.

So `{:#?}`, `{:x?}` and a width of a `dyn Debug` made elsewhere apply, as
its `impl` is given the options; ADR 0137's refusal of them is gone, and so
is `can_apply`'s. So is "a `dyn` of a value with a destructor": the pair
drops its value through its `impl`, as any pair's does. A struct whose last
field is a `dyn Debug`, and a counted `Rc<dyn Debug>`, stay refused.

Case C of [0262](0262-when-rust-and-js-disagree.md): `fmt` runs where Rust
calls it. Snapshots that change: `boxed_dyn_debug`,
`impl_named_like_a_function`, `dyn_shapes_shared`, `default_method_drops`
and `dyn_drops` show their pairs through `impl`, and in the last two the
unwind blocks regroup around the call, which now runs where it's shown;
`weak_debug` and `refcell_variant_derives` rename a derived field's
temporary. Held by the corpus case `dyn_debug_shown_where_rust_calls_fmt`, in
both builds and through `METAMORPHIC=all` with the cases above. Mutations
`dyn-debug-made-a-pair-where-made` (`lower/mir/rvalues.ts`),
`held-dyn-debug-made-a-pair` and `held-dyn-debug-always-spilled`
(`lower/mir/temporaries.ts`), `writer-dyn-debug-made-a-pair`,
`builder-dyn-debug-made-a-pair` and `boxed-dyn-debug-paired-again`
(`lower/mir/writer.ts`), `placeholder-dyn-debug-made-a-pair`
(`lower/mir/std_calls.ts`), `dyn-debug-pair-shown-whole` and
`dyn-debug-options-dropped` (`lower/display/debug.ts`), and
`builder-step-shown-plain` (`lower/display/written.ts`).
`mir-dyn-debug-made-early`, `mir-writer-fields-plain`, `debug-dyn-twice` and
`mir-pretty-dyn-debug-plain` went with their code.
