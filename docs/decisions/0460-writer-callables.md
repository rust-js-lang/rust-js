# 0460. A callable that writes to a `Formatter` is a writer, and a writer's options are its crate's

Status: Accepted. Extends [0054](0054-display.md),
[0070](0070-function-values.md) and [0137](0137-pretty-debug.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)): the same text, and
the same widths applied and ignored.

## Context

heck converts a string's case in a `Display`, by one function given what
writes each word and what writes between them:

```rust
fn transform<F, G>(s: &str, mut with_word: F, mut boundary: G, f: &mut fmt::Formatter) -> fmt::Result
where
    F: FnMut(&str, &mut fmt::Formatter) -> fmt::Result,
    G: FnMut(&mut fmt::Formatter) -> fmt::Result,
```

given `lowercase`, a writer function, and `|f| write!(f, "-")`. A writer
function was a function that returns its text and takes no `Formatter`
(ADR 0054), but a closure that writes wasn't, and calling a callable
given the `Formatter` was refused. So was `{}` of `c.to_lowercase()`.

Its consumer's `{:>12}` of heck's `AsKebabCase` then called heck's `fmt`
with options heck's writers don't take: whether a writer takes options
was its caller's crate's answer (ADR 0137), not its own.

## Decision

**A callable whose arguments hold a `&mut Formatter` and that returns a
`fmt::Result` is a writer, as a writer function is: it takes no
`Formatter` and returns its text, and calling it is what it writes.**

```js
function transform(s, withWord, boundary) {
  let f = "";
  for (const [i, word] of s.split(/\p{White_Space}+/u).filter((word) => word !== "").entries()) {
    if (i > 0) {
      f += boundary();
    }
    f += withWord(word);
  }
  return f;
}

function kebabDisplay_fmt(kebab) {
  return transform(kebab[0], lowercase, () => "-");
}
```

1. **Which are writers is `formatter_param`'s**: a function's by its
   signature, a closure's by its own, and `callable_formatter` of a
   call of a callable by its arguments' types and its result's.
2. **A writer closure's body is a writer's**, `let f = ""`, given its
   `Formatter`'s options after its arguments where the crate's writers
   take them.
3. **`with_word(word, f)` is `f += withWord(word)`**: the arguments'
   tuple made for it holds the `Formatter`, which the call isn't given,
   and the writer's options are, where it has them.
4. **`{}` of a `char`'s case mapping** is the `char`s it hasn't given:
   `c.toUpperCase()` made where it's shown, `$restStr(upper)` of one
   stepped.

### A writer's options are its crate's

**Whether a writer function takes its `Formatter`'s options is its own
crate's answer, `takes_options`: a library's manifest says it,
`writers_take_options`.** A consumer that gives options gives none to a
library's writer that takes none, whose `write!` ignores a width, as
Rust's does; one that has none gives a library's writer that takes them
`undefined` in their place, before the dictionaries after them. A
callable may be any crate's writer, and is given the writer's own.

A consumer's writer may now call a library's writer function given its
`Formatter`, as it calls its own crate's: `is_rust_fn` answers both.

## Why

- **It's how a person writes it in JS**: a function that returns text,
  called where its text goes, `f += withWord(word)`.
- **It's one rule**: what a writer function is, a callable is too.
- **It's exact**: the `writer_callables` corpus case passes a writer
  function and closures, one that captures and writes twice, and one
  that pads to its `Formatter`'s width, and shows case mappings, made
  and stepped, against native Rust; heck's probe converts every case
  the same as native heck, a `{:>12}` of a library's `Display` too; and
  `crates.test.ts` gives a library's writers options as each takes them.
