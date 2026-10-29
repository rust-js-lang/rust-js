# 0642. A place found in a string is JS's: `indexOf`, `slice`, `startsWith`

Status: Accepted. Amends [0138](0138-string-byte-counts.md), whose `find`,
slices, `len()` and `strip_prefix` were helpers that count UTF-8 bytes
wherever they're written, and [0247](0247-js-string-methods.md), whose
`js::string` is now for JS's own indexes alone.

Case ([0262](0262-when-rust-and-js-disagree.md)): A, the program can't
observe it: an offset only a place in the string it was found in reads
the same text in UTF-16 units as in UTF-8 bytes, and is never seen as a
number. And C where it can: an offset printed, returned, stored, compared
with a number, used on another string, or stepped by what wasn't found,
keeps ADR 0138's bytes, paid for in output.

## Context

React takes an owner stack apart with JS's own methods,
`shared/ReactOwnerStackFrames.js`:

```js
  if (stack.startsWith('Error: react-stack-top-frame\n')) {
    stack = stack.slice(29);
  }
  let idx = stack.indexOf('\n');
  if (idx !== -1) {
    stack = stack.slice(idx + 1);
  }
```

Its port writes std's, `strip_prefix`, `find`, `&stack[idx + 1..]`, which
ADR 0138 made exact helpers: `$stripPrefix(stack, ..)`, `$find(stack,
'\n')`, `$strSlice(stack, (idx + 1) >>> 0)`. Each counts bytes, as Rust's
answer does, though nothing in the function ever sees a byte count: each
offset is found in `stack` and only cuts `stack`. And ADR 0247's
`js::string::{index_of, slice}` gave JS's methods by JS's indexes, a second
way to search a string and cut it, where the port has to choose.

## Decision

**std's `find`, `rfind`, slicing, `len()`, `strip_prefix` and
`strip_suffix` are JS's `indexOf`, `lastIndexOf`, `slice`, `length`,
`startsWith` and `endsWith` wherever the program can't tell UTF-8 bytes
from UTF-16 units; ADR 0138's helpers, exact in bytes, everywhere else.**
One question decides it, of each MIR local: whether it's only a place in
the string it was found in, `offsets::in_units`, answered by an analysis of
the body's MIR (`src/lower/mir/offsets.rs`), which every lowering of these
methods asks:

```rust
pub fn format_owner_stack(stack: &str) -> String {
    let mut stack = stack;
    if let Some(rest) = stack.strip_prefix("Error: react-stack-top-frame\n") {
        stack = rest;
    }
    if let Some(idx) = stack.find('\n') {
        // Pop the JSX frame.
        stack = &stack[idx + 1..];
    }
    let idx = stack
        .find("react_stack_bottom_frame")
        .and_then(|idx| stack[..idx].rfind('\n'));
    if let Some(idx) = idx {
        stack = &stack[..idx];
    } else {
        return String::new();
    }
    stack.to_string()
}
```
```js
export function formatOwnerStack(stack) {
  if (stack.startsWith("Error: react-stack-top-frame\n")) {
    stack = stack.slice(29);
  }
  const idx = stack.indexOf("\n");
  if (idx !== -1) {
    // Pop the JSX frame.
    stack = stack.slice(idx + 1);
  }
  const at = stack.indexOf("react_stack_bottom_frame");
  const idx$1 = at !== -1 ? stack.slice(0, at).lastIndexOf("\n") : -1;
  if (idx$1 !== -1) {
    stack = stack.slice(0, idx$1);
    return stack;
  }
  return "";
}
```

- **A place is made** by `find(p)` or `rfind(p)` of a `char`, a `&str` or
  a `String`, stepped past what was found by its length, a literal's (`+
  1` past `'\n'`, written in units: `+ 1` past `'é'`'s two bytes) or the
  pattern's own `len()` or `len_utf8()`, `p.length`; by a string's `len()`,
  `length`; or `0`.
- **A place is used** to slice the string it's a place of, `&s[a..]`,
  `&s[..b]`, `&s[0..b]` or `&s[a..s.len()]`, `s.slice(a, b)`; to compare
  with another place of it, or `0`; or as its `Option`'s test.
- **A place is of one string's value.** The analysis follows the value
  each local holds through the body as it runs, by where it was made: a
  variable given another string, `s = t`, holds another, and a place of the
  one before slices it in bytes. A place found in a prefix, `&s[..i]`, is
  one of `s` too, its units the same; one found in a suffix isn't. A string
  that may change unseen, through a `&mut` or a pointer, holds none.
- **An `Option` of a place, only tested and taken apart, is `-1` where
  it's `None`**, as `indexOf` gives it: `at !== -1`. Given to a closure the
  body makes, `and_then`'s or `map`'s, that takes it as a place, the
  closure is told so (`offsets::Seed`), and the call is `at !== -1 ? f(at)
  : -1`, `map`'s `: undefined`.
- **A strip whose `Option` is only tested and taken apart is a test and a
  cut**, which count nothing: `s.startsWith(p)`, and where it holds
  `s.slice(n)`, `n` a literal's length in units or `p.length`; of a suffix,
  `s.endsWith(p)` and `s.slice(0, -n)`. `s` and `p` are read where they're
  tested and cut, where what holds them still does, or held first. A cut
  given to a local read once is read there: `stack = stack.slice(29)`.
- **Anything else of an offset is ADR 0138's**, in bytes, every local it
  meets with it: a place printed, returned, stored, compared with another
  number, used on another string, stepped by what wasn't found (`at + 3`
  past an `'a'`), `unwrap()`ed, `?`'d, or a range of two places found apart,
  `&s[a..b]`, which may run backwards.

## Why

- **Bytes and units count the same text.** A char boundary `b` of a
  string, in bytes, is `u(b)` in its UTF-16 units, one to one and in the
  same order; `u(0)` is `0` and `u(len())` is `length`. `indexOf` finds the
  same first match `find` does, at `u` of it, as a pattern of whole
  characters matches only at whole characters in a string that's well
  formed, as a Rust string is. Past a match of `p`, `b + p.len()` is a
  boundary again, `u(b) + p.length`. So `s.slice(u(a), u(b))` is `&s[a..b]`'s
  text, `a < b` is `u(a) < u(b)`, and `a == 0` is `u(a) == 0`.
- **Neither can panic.** A place is in bounds and on a boundary: of what's
  found, `b + p.len() <= len()`. A slice from a place to the end, or from
  `0` to one, can't run backwards; a range of two places found apart may,
  and Rust's panic there is in bytes, so it's the helper's.
- **No step wraps.** A place is at most the string's `length`, below
  `2^32 - 1` (ADR 0632), so `at + 1` is written without `>>> 0`.
- **It's the JS a person writes**, React's own: `stack.slice(29)`, `idx !==
  -1`, no import.

## `js::string`'s search and slice

`js::string::index_of`, `last_index_of` and `slice` stay: they're the
binding of JS's own API, whose indexes are JS's, for an index a JS API
gives or takes, `selectionStart`, `setSelectionRange`, a `RegExp`'s
`lastIndex`, or `slice`'s from the end, `slice(0, -1)`. They're not a
second way to search a Rust string and cut it at what's found: that's
std's, which reads as JS's now wherever the number isn't seen. The guide
says which a program writes when (`for-js-developers/collections.md`).

## Alternatives

- **Units everywhere, `length` and `indexOf` for every `len()` and
  `find`**: ReScript's, and a different answer for any text that isn't
  ASCII wherever the number is seen, ADR 0138's first alternative.
- **`-1` as every `Option<usize>`'s `None`**: an `Option`'s representation
  is its type's, `representation.rs`'s, which every function's signature
  and every caller shares. A place found is one local's choice, of what it
  meets in its body, so the analysis decides it where it decides units.
- **The rewrite in tidy, of `$strSlice(s, $find(s, p))`**: JS knows neither
  which string a number is a place of nor what it meets; MIR does.

## Consequences

- A port's search-and-cut code reads as its original: React's owner stack,
  the playground's `jsName`, `path.endsWith(".rs") ? path.slice(0, -3)`.
- `string_bytes`, `const_blocks_and_let_guards`, `option_combinator_drops`
  and the `strings` and `playground` snapshots read JS's methods. The
  readability baseline's `while (true)` and `return;` grow by the new
  corpus cases' `while let` and `?`, not by any program's JS reading worse.
- A place used any other way keeps the helpers, in the corpus's
  `string_places_kept`: the analysis is of one body, so a place given to a
  function, or returned, is in bytes, and so is one stepped by a number it
  can't tie to what was found.
- std's bodies, compiled into `@rust-js/std`, are asked the same question;
  none of theirs changes.

## What holds it

- The corpus, each case run natively and as JS on text of one to four
  bytes a character: `string_places` (React's owner stack, steps past what
  was found, a loop of finds, prefixes, strips of literals and of what's
  given), `string_places_kept` (each way an offset is seen) and
  `string_bytes`, `string_slice_boundary`, `string_slice_bounds`.
- `test/lowering-string-places.test.ts`: the owner stack's JS, steps, strips,
  `map`, each way bytes are kept, a place of a string a variable held
  before, of a suffix, of two places found apart, which panics, and a strip
  read after its string changed.
- Mutations in `scripts/mutations/lower/mir/offsets.ts`, and of their
  lowering in `std_types/text.ts`, `mir/rvalues.ts` and `mir/std_calls.ts`:
  each rule and each soundness condition, caught.
- `test/architecture.test.ts` holds `offsets.rs` a question, which emits
  nothing, and which state of MIR's lowering its askers read.

## Amendment: an integer's `try_from` only tested is its range test

javascript-algorithms' `recursiveStaircaseDP` writes `if (stairsNum < 0)
return 0;`, and the idiomatic Rust is `let Ok(stairs_num) =
usize::try_from(stairs_num) else { return 0.0; };`, which printed `const
value = $tryFromInt(stairsNum, 0, 4294967295); if (value.TAG !== "Ok")
..` and `value._0` after (#366). **An integer's `try_from` whose `Result`
is only tested and its `Ok` taken is a cut too**: its test is the range
test, of each bound its type can pass, `stairsNum < 0` of an `i32` into a
`usize`, `n < 0n || n > 255n` of an `i64` into a `u8`, `false` of one
that can't fail; its `Ok`'s number is the number, a BigInt or a number
where the two types' differ, as `$tryFromInt` makes it.

- **Which `Result`s** (`mir/tried_ints.rs`, kept as `LocalFacts::tried_int`): one written once, by
  the call, never the return place, read only by its discriminant, a
  `let`'s mention of it, and its `Ok`'s field: never its error, `Err(e)`,
  nor whole, which keep `$tryFromInt`. Not a `NonZero`'s, whose `0` is its
  error too (ADR 0177).
- **A cut says which variant it's the value of**: `Value::Cut` and
  `State::cuts` keep it, a strip's `Some`, a `try_from`'s `Ok`, the
  first. Its test is still that it's the second, an `Err` here, as
  `switch_test` reads a `Tested`.
- **What may change, `counter.at`, is held once**, `const value =
  counter.at;`, as a strip's string is; a local nothing assigns is read
  where it is.
- **A comparison of integers is negated by its converse** (`js::Known::Integers`,
  `Expr::negated_test`): an `Ok` arm's test, `n >= 0n && n <= 255n`, not
  `!(n < 0n) && !(n > 255n)`. An `f64`'s isn't, as `!(x < y)` holds of a
  NaN where `x >= y` doesn't.

Case N: the test and the number are what `$tryFromInt` made; its error
was never read. Held by `test/corpus/try_from_int_tested.rs` against
native Rust: a `let`-`else`, an `if let`, a `match` of `Err(_)`, one into
a wider type that can't fail, each bound of `i64`, `u64` and `i128`, an
error read, a `Result` given back whole, and a variable or a field
changed before its `Ok` is read. Mutations `tried-int-*`
(`mir/local_facts.ts`, `mir/tried_ints.ts`, `std_types/number/generic.ts`), `cut-of-the-second-variant`
(`mir/places.ts`) and `integers-negated-by-not` (`js/make.ts`). The guide's
numbers section, in `for-js-developers/basics.md`, shows it.
