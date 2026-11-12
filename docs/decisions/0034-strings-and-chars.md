# 0034. String methods are JS's; a `char` is a one-character string; `format!` is `+`

Status: Accepted; `format!` is a template literal now ([0066](0066-template-literals.md)), and byte counts are counted ([0138](0138-string-byte-counts.md)). Extends [0023](0023-strings-references-shared-state.md). Extended by [0652](0652-consecutive-appends-are-one.md): two appends in a row are one `+=`.

Case: A, D, C ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A `String` or `&str` is a JS string (ADR 0023), but rust-js knew only a few
of their methods: `trim`, `is_empty`, `==`, `+`, `to_string`. Every part of
the playground left to port works with paths and messages: `split("/")`,
`ends_with(".rs")`, `replace`, `join`, and template strings. `format!` and
`char` were errors.

Rust and JS agree on what most string operations mean, but not on how to
count: Rust's `len()` and `&s[a..b]` count UTF-8 bytes, and JS's `length` and
`slice` count UTF-16 units. ReScript's strings are JS's, with JS's counting.

## Decision

**A method that means the same in both is the JS one:**

| Rust | JS |
|---|---|
| `s.starts_with(p)`, `s.ends_with(p)`, `s.contains(p)` | `s.startsWith(p)`, `s.endsWith(p)`, `s.includes(p)` |
| `s.replace(a, b)`, `s.replacen(a, b, 1)` | `s.replaceAll(a, b)`, `s.replace(a, b)`: `b` the text it is, a `$` of it doubled, `"$$&"`, or of one not written out a function's, `() => b`, as JS reads `$&` in a string as what's matched. (Amended: `b` was given as it is, so `"$&"` was the match; `replacen` was an error.) Another count of `replacen` is an error still. |
| `s.to_uppercase()`, `s.to_lowercase()` | `s.toUpperCase()`, `s.toLowerCase()` |
| `s.trim_start()`, `s.trim_end()`, `s.repeat(n)` | `$trimStart(s)`, `$trimEnd(s)`, `s.repeat(n)`: Unicode's White_Space (ADR 0183). (Amended: it was `s.trimStart()`, `s.trimEnd()`.) |
| `s.strip_prefix(p)`, `s.strip_suffix(p)` | `$stripPrefix(s, p)`, `$stripSuffix(s, p)`: an `Option` (ADR 0030) |
| `s.split_once(p)`, `s.rsplit_once(p)` | `$splitOnce(s, p)`, `$rsplitOnce(s, p)`: an `Option` of the two sides |
| `s.split(p)` | `s.split(p)`, an array: for `for`, `collect()`, `last()` (`.at(-1)`), `count()` |
| `parts.join(sep)` | `parts.join(sep)` |
| `s.push_str(t)`, `s.push(c)` | `s = s + t` |
| `s.clone()` | `s` |

- **A pattern must be a string or a `char`.** A closure or a set of `char`s is
  an error, for now.
- **`push_str` and `push` assign**: JS strings don't change, so the variable
  gets a new string. The string must be in a place rust-js can assign to,
  a variable or a field, not behind a `&mut String` parameter.

**A `char` is a string of one character**: `'/'` is `"/"`. It goes wherever
a string pattern does (`split('/')`), and `==` and `to_string` work as on
strings.

**A string literal in a `match` is `===`**, since JS compares strings by
content: `"abc" | "stats.rs" =>` tests `s === "abc" || s === "stats.rs"`.
In `Some("ab")`, `top === "ab"` already rules out `None`, so there's no
`!= null`.

**`format!` is the pieces joined with `+`**, from the template rustc builds,
as `panic!` already was (ADR 0026):

```rust
format!("{name}: {} item{}", n, if n == 1 { "" } else { "s" })
```

```js
name + ": " + String(n) + " item" + (n === 1 ? "" : "s")
```

`format_args!` keeps its values in two `let`s of its own, a tuple and then an
array. rust-js recognizes the whole block, as it does `?`, and writes each
value where the template shows it: `return t.toFixed(0) + " ms";`. The
values are operands like any others, so a call in one doesn't put the calls
before it in `const`s. Some values still go in a `const` first, which is
then used in the string:

- **A value the template shows twice**, unless it's a variable or a
  constant, so that it runs once. So is one `{:?}` shows by its parts, as
  it does an `Option`, a `Result` or a tuple: `const arg = first_dup(s);`
  and then `arg == null ? "None" : ..`. Values before it that have effects
  go in `const`s first, in their order, and so do those that aren't places
  before one with effects: `v.len()` before `v.pop()` is read before it
  pops. A `const` made already, a call's result, stays as it is: nothing
  can change it. (Amended: it was read after, a length too short.)
- **Values shown in another order than they're written,** when one of them
  has effects: `format!("{1} {0}", tick(&c), tick(&c))`. Each value that
  isn't a place goes in a `const`, in the order Rust runs them.

  Rust puts named arguments like `{name}` after the others, but reordering
  values that have no effects changes nothing. A place is safe too: every
  value is borrowed until the string is made, so none can change another.

**Byte counts are an error**: `len()` of a string, and indexing or slicing
one by a range, say that JS counts differently. `is_empty()` works.

## Why

- **The JS reads like hand-written JS**, with the names every JS programmer
  knows, and no helper for what JS already does.
- **It's what the playground needs**: its paths, file names and messages.
- **Refusing byte counts is honest.** Mapping `len()` to `length` would agree
  for ASCII and quietly disagree otherwise. An error says so where it matters.

## Alternatives

- **Counting bytes in JS** (`new TextEncoder().encode(s).length`): faithful,
  but slow, and slicing by bytes would have to encode and decode each time.
- **A `char` as its code point, a number**, as ReScript and Scala.js do: good
  for arithmetic on characters, but then `split('/')` needs a conversion, and
  a `char` in a message prints as a number.
- **`s.split(p)` as a real iterator** (a JS generator): lazy, like Rust's, but
  `for`, `collect` and `last` are what programs do with it, and an array
  serves them all.

## Consequences

- `trim_*` removes Unicode's White_Space, and `char`s compare by code point,
  as Rust's do (ADR 0183). (Amended: they were JS's whitespace and UTF-16
  units, which ordered a few characters above U+FFFF differently.)
- `{}` takes strings, `char`s, integers, `bool`s and floats, and formatting
  options (`{:>8}`, `{:.2}`, `{:#x}`) are ADR 0058's.
- `lines`, `split_whitespace`, `parse` and `char`'s questions are ADR 0063's.
- Not yet: `find`, `chars`, `char_indices`, `split` beyond those uses, and
  patterns that are closures.

## Amendment: `bytes()` is an array source

A string's `bytes()` (2026-10-15) is an iterator over an array, its UTF-8
bytes, as `slice::Iter` is: recognition's `array_source` says so, the one
list, so a clone of one is what it has left, `$iter` of it (ADR 0181), as
a slice's is, and `steps_an_array` asks no list of its own. The corpus's
`str_bytes.rs` clones one part-stepped; rustc's
`iterators/bytes-iterator-clone-12677.rs` passes.

## Amendment: `Arguments::as_str()` is the text

`fmt::Arguments::as_str()` (2026-10-21) is `Some` of the text the
arguments are already in JS: its contract promises `Some` of a literal
alone and says the rest "is not guaranteed, and should not be relied upon
for anything but optimization", so `Some` of any is within it, and no
program can tell (ADR 0262's first case). anyhow's `format_err`, which
keeps a literal as a `&'static str` and formats the rest, takes its first
way for both, the same string either way. The corpus's
`fmt_arguments_as_str.rs` holds it against native Rust, comparing the
literal cases and a placeholder's text alone, since which way a
placeholder's takes the contract leaves open: native Rust formats it,
rust-js keeps it, the same string.

## Amendment: `as_str()` is `None` of arguments made with a value (#1091)

The amendment above held `Some` of any text within `as_str()`'s contract,
"no program can tell". A program can: `{:?}` of
`format_args!("{count} items").as_str()` prints `None` natively and
`Some("0 items")` in JS, and anyhow's `format_err` keeps a formatted
message as a literal. Rust's results change, so Rust's behavior is kept
(ADR 0262); that amendment is superseded.

- **Where the body made the arguments, its maker says which**: MIR's
  `Arguments::from_str("plain")`, `from_str_nonconst`, which rustc makes of
  a constant argument it writes into the literal, `format_args!("{} items",
  3)`, or a constant, gives `Some` of the text; `Arguments::new(..)`, made
  with a value to format, gives `None`, `undefined`, as native Rust's.
- **Of arguments made in another function**, a parameter's, nothing says
  which, as the JS value is their text either way: refused, "`as_str()` of
  a `fmt::Arguments` made in another function". anyhow's `format_err`,
  which asks it of its parameter, is refused there until the value carries
  which it is.

The corpus's `fmt_arguments_as_str.rs` prints each way's own text, against
native Rust; `test/diagnostics.test.ts` holds the refusal. Mutations
`fmt-as-str-of-formatted` and `fmt-as-str-of-elsewhere-taken` put each
old answer back.

## Amendment: `chars()` and `char_indices()` are lazy (#1186)

`chars()` was `Array.from(s)` and `char_indices()` an array of its
pairs, each made whole wherever one `char` was read. Both are lazy now:
JS's own string iterator where only the front is stepped, and the
runtime's `$chars(s)` or `$charIndices(s)` otherwise
([1117](1117-a-strings-chars-are-lazy.md)).

## Amendment: an `Arguments` given on is a function of its text (2026-11-13, #1565)

A `format_args!` was its text, made where the macro is written. That's
right where std formats it at once, in `format!`, `print!`, `write!` and
`panic!`. It's wrong where the `Arguments` is a value given on: passed to
a function, stored, or returned. Rust formats such a value where it's
written, inside the callee, and as often as it's written. In JS its
writers ran once, before the callee's first statement:
`said(format_args!("{:?}", noisy))` printed `<noisy>said: N` where native
Rust prints `said: <noisy>N`. A value written twice ran its writers once, a
writer's panic came before the callee's output, and a `Cell` a writer
reads was read early. The metamorphic net's `loud` rewrites found it, as
their `metamorph_loud(format_args!(..))` counts before it formats. Results
change, so Rust's behavior is kept (ADR 0262). The lead approved the design
on the issue, on the user's behalf.

**One fact from MIR, `mir::arguments_use`, says which an `Arguments` a body
makes is.** It's `InPlace`, its text as before, where each use of it is std's
by value (`format`, `_print`, a `write_fmt`, `panic_fmt`), its `as_str()`, or
a holder std alone is given, `assert_eq!`'s `Some(args)`. Otherwise it's
`GivenOn`: a function of its text, `said(() => noisyDebug_fmt(value))`. One
form per place: a parameter, a field, or any place the body didn't make its
text in holds the function (`holds_text`). std's formatting, given one by
value, calls it where it formats (`arguments_formatted`), and so does
`{}` of one (`display_shown`). The call is taken to print or fail where any
of the crate's writers may, as a `dyn`'s value is.

**Only the formatting is deferred, not the operands.** `format_args!`
evaluates its operands where it's written, in order. The function captures
them already evaluated, each held in a `const` but a variable or a
constant, `const value = bump(effect)`. Its writers alone run when it's
called. An operand's effect happens once, where the macro is written, and
a `Cell` a writer reads shows what it holds when it's written.

A function of one whose writer may fail makes its text piece by piece, and
what it made before the `Err` goes with it, `$fmtWritten`, as one made in
place writes it: `put(out, format_args!("a{:?}b", bad))` leaves `[aB`.
`fmt_failures.rs` takes a function giving a write's result that formats an
`Arguments` it didn't make to fail where a dictionary's may. `as_str()` of a
literal given on calls its function. `as_str()` of a parameter's stays
refused (#1091's amendment).

No field of a library's contract changes: an `Arguments` crossing a
library boundary is a function on both sides, made by the same compiler,
so ADR 1129 asks no row.

`format!`, `print!`, `write!` and `panic!` are byte for byte as they were.
Of the corpus, `fmt_arguments_as_str` and `fmt_result_map_err` change (their
`Arguments` given on), and so does `user_fmt_traits`: strum's
`Display::fmt(&format_args!(..), f)` lends it to std's `Display`, a use other
than as text, so its operand is held. Corpus case
`format_args_given_on_formatted_where_written` runs against native Rust in
both builds. It covers a callee that prints before formatting, one that
formats twice, one that never does, one given through two functions, an
operand with an effect, a `Cell` set between making and formatting, one
held in a struct, one made in a loop, a call operand with an effect
formatted twice, a writer that panics (caught), one that fails through
`write_fmt`, and `as_str()` of a literal and of one made with a value.
test/architecture-mir.test.ts holds `arguments_use` the one answer. The
metamorphic net's `loud` rewrites of #1533's and #1557's cases now run as
Rust does. Mutations: `given-on-arguments-made-whole`,
`given-on-operands-made-late`, `given-on-arguments-uncalled`,
`given-on-failing-partial-lost`, `arguments-given-to-crate-taken-in-place`,
`assert-message-given-on` and `as-str-lent-given-on`;
`fmt-as-str-of-formatted` is refitted. Readability: role fallbacks 270.9
to 271 in 10,000 lines, blessed: an operand held where it's made, of a
`format_args!` given on, is named `value` where its maker says nothing of
it, `const value = [1]` of `Noisy(1)`.
