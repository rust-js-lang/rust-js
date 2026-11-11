# 0137. `{:#?}` is pretty Debug, and a writer is told whether

Status: Accepted. Extends [0054](0054-display.md), [0060](0060-debug.md) and [0136](0136-common-std-methods.md). Its `alternate` parameter is superseded by [0143](0143-formatter-options.md): writers take an `options` object, `{ alternate: true }`.

Case: C, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

`{:#?}` was an error everywhere: of a `Vec`, an `Option`, a derived `Debug`,
anything. It's what Rust programs print values with more than anything but
`{:?}`:

```
Point {
    x: 1,
    tags: [
        2,
        3,
    ],
}
```

Rust's rule composes: each part goes on a line of its own, indented by four
spaces, and ends with a comma; a part that's pretty itself has its own lines
indented too. A `Formatter` carries whether it's alternate into each `fmt`
it's given to, and `f.alternate()` asks. A `write!`'s own placeholders are
plain, whatever its `Formatter` is.

Re-formatting the plain string afterwards would be wrong: a hand-written
`fmt` that writes `[5]` with `write!` shows `[5]` under `{:#?}` too.

## Decision

**A crate that shows a value with `{:#?}`, or asks a `Formatter` if it's
alternate, gives each of its writers (ADR 0054) whether it's pretty, a
parameter after its value, `alternate`; and each part is shown as pretty as
what it's part of:**

```js
function pointDebug_fmt(point, alternate) {
  const shown = String(point.x);
  const shown$1 = $debugStr(point.label);
  return alternate
    ? $pretty("Point {", [`x: ${shown}`, `label: ${shown$1}`], "}")
    : `Point { x: ${shown}, label: ${shown$1} }`;
}
```

- **`$pretty(open, items, close)`** is the rule: each item on a line of its
  own, its lines indented by four spaces, ended by a comma; none, `[]`. A
  `finish_non_exhaustive()`'s `..` is a line of its own, with no comma.
- **Std's types are pretty as Rust's are**, where `{:#?}` asks, or as a
  writer's `alternate` says: `Some(..)`, tuples, `[..]`, `{..}` of a map or
  a set, `Ok(..)`, `RefCell { value: .. }`, and serde_json's `Array [..]`
  and `Object {..}`.
- **A derived `Debug`'s fields and a builder's arguments** (ADR 0136) are
  shown as pretty as their writer: the `&dyn Debug` they're given, which is
  the string it shows (ADR 0060), is made so. A `write!`'s placeholders
  are plain, as Rust's are.
- **A `&dyn Debug` made in a format's arguments for its `{:#?}`**, as
  `dbg!` makes one, is made pretty. `{:#?}` of one made anywhere else,
  which is a plain string already, is an error.
- **A writer that hands its `Formatter` on**, `self.0.fmt(f)` or a function
  of the crate's own, hands on its `alternate`; and a dictionary's `fmt`
  takes one, std's and the crate's.
- **`{:#}` of the crate's own `Display`** is given `alternate` too, for its
  `f.alternate()` to ask.
- **A crate with no `{:#?}`, and no `f.alternate()`, gets the JS it got**:
  its writers take no `alternate`. `{:#}` and `{:#x}`, alternate
  placeholders of no `Debug` argument, aren't a reason to.
- **A plain call is given no `alternate`** where nothing comes after it:
  `pointDebug_fmt(p)`.

## Why

- **It's exact**: the same lines, indents and commas as Rust's, from the
  same parts, compared with native Rust by the `pretty_debug` corpus case;
  and a `write!` stays as it's written.
- **It's the JS a person writes**: a flag passed down, and one helper for
  the rule, only where the crate asks for it.

## Consequences

- `{:#?}` and `f.alternate()` compile.
- A `Debug` builder kept in a variable is still an error (ADR 0136).
  (Amended: it compiles, [ADR 0532](0532-a-debug-builder-held-in-a-variable.md).)
- The rustc suite compiles a test natively and as JS as at its own path,
  for what `file!()` and `dbg!` print to be the same.

## Amendment: a library's `{:#?}` or `{:x?}` of a consumer's value (#1331)

Case ([0262](0262-when-rust-and-js-disagree.md)): C, results change, so
Rust's behavior holds. Whether the crate's writers take a `Formatter`'s
options was one answer for the whole crate, asked of its own THIR: its
`{:#?}` and `{:x?}`. A library's generic function that shows what it's
given so, `pub fn pretty<T: Debug>(held: &T) -> String { format!("{:#?}",
held) }`, gives the consumer's dictionary `TDebug.fmt(held, { alternate:
true })`. The consumer's own code showed nothing so, so its derived
`plainDebug_fmt(plain)` took no options. It printed `Plain { n: 2 }` where
native Rust prints `Plain {\n    n: 2,\n}`, and `Plain { n: 255 }` where
`{:x?}` shows `Plain { n: ff }`.

**A writer takes options where its crate's writers do, or where its type
reaches a parameter a function shows with them** (`analysis/shown_params.rs`,
an Instances fact of [1115](1115-what-generic-code-needs-of-its-type-parameters-has-one-catalogue.md)'s catalogue):

- Each function's type parameters it shows pretty or in hex are found:
  those its own placeholders show, `{:#?}`, `{:x?}`, `{:#x?}`, and those it
  gives a function that shows them so, to a fixed point through its calls.
  A library's are its manifest's `shown_pretty` and `shown_hex` marks of
  each function's type parameters (library ABI 27).
- Each type the crate gives such a parameter at a call it sees, each type
  it's made of, and each field of the crate's own, which its `Debug`
  shows, is shown so. Its writer, and a std type's dictionary of it, takes
  the options. A writer of one shown in hex applies a placeholder's options
  to its integers, `$debugInt`.
- A type that reaches no such parameter keeps its plain writer,
  `untouchedDebug_fmt(untouched)`. Every app's derived `Debug` isn't given
  the options its libraries might show.

This was decided for the user by the coordinator. The alternative, making
every consumer's writer take options whenever any library shows anything
so, would add the plumbing to every derived `Debug` in every app.

- A library that gives one of its own types to such a parameter makes
  each of its writers take options, as its manifest's
  `writers_take_options` says of them all: its consumers count a writer's
  arguments by it, and a writer's options come before its dictionaries.
- A writer this build compiles, its own or a body of another crate's
  (ADRs 0561, 0914), answers by this build's facts
  (`compiled_writer_takes_options`). A writer it calls of a library's
  answers by that library's manifest, as before.
- Not covered: a value given as a `&dyn Debug` to a library's function that
  shows it with options. A `dyn` reaches no type parameter, so its writer
  takes options only where the crate's do.

Held by `test/crates/pairs/library_shows_a_consumers_value_with_options`,
against native Rust: `{:x?}`, `{:#?}` and `{:#x?}`, a chain of two library
functions, a field's type reached only through its struct, a `Vec<u32>`'s
std dictionary, and an untouched type whose writer stays plain. Mutations
`shown-params-of-no-calls` and `shown-fields-unreached`
(`lower/analysis/shown_params.ts`), `shown-params-unexported`
(`lower/library.ts`), and `compiled-writer-shown-ignored` and
`shown-hex-unformatted` (`lower/display.ts`).

## Amendment: a `dyn Debug` made elsewhere is shown with its options (#1335)

`{:#?}` of a `&dyn Debug` made anywhere but its format's arguments was
refused: it was the string it showed where it was made. A `dyn Debug` stored
or passed is now the pair `{ value, impl }` (ADR 0060, amended), so `{:#?}`,
`{:x?}` and a width of one are given to its `impl`'s `fmt`, as a `T: Debug`'s
are. Within a crate, a `{:#?}` anywhere makes every writer take options, as
before.

A library's function that shows a `dyn Debug` it's given with options is the
case "Not covered" above. The user decided (2026-11-11) that a type made a
`dyn Debug` gets an option-taking writer through #1331's writer facts, so
only types used so pay. The coordinator, for the user, chose the precise
form of it, with the same precision as #1331: a library's manifest says
whether it, or a library it uses, shows a `dyn Debug` it's given pretty or in
hex (`dyn_debug_pretty`, `dyn_debug_hex`, from `ShownParams::dyn_debug`). A
consumer marks a type it makes a `dyn Debug` outside a writer with what its
libraries show of one, and only where one does. That type's writer, and its
std dictionary, then take those options. A crate whose libraries show no
`dyn Debug` with options gets the JS it got; a type made a `dyn Debug` for a
`{:#?}` placeholder of the crate's own, which `format_args!` binds first, is
counted too. Held by
`test/crates/pairs/library_shows_a_consumers_dyn_debug_with_options` against
native Rust: `{:#?}`, `{:x?}` and `{:#x?}` of a consumer's struct, a field's
type reached only through it, a `Vec<u32>`'s and an `Option`'s std
dictionaries, a `Vec<Box<dyn Debug>>` the library holds, and an untouched
type whose writer stays plain. Mutations `dyn-debug-made-unshown`
and `library-dyn-debug-unrecorded`
(`lower/analysis/shown_params.ts`), and `manifest-dyn-debug-hex-dropped`
(`lower/library/exports.ts`).
