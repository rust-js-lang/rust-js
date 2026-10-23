# 0639. `concat_text` of any value is what may throw; whether a call threw is `try`

Status: Accepted. Amends [0310](0310-any-value-as-js-reads-it.md), whose
`concat_text` gave a `String` of every value, and
[0637](0637-a-match-of-a-throwing-call-is-try-catch.md), whose `try` was
of a `match` whose arms are statements. Amended below: `"" + value`, empty
text first.

Case ([0262](0262-when-rust-and-js-disagree.md)): C, N. Results would
change, so Rust's is kept: `value + ""` of a symbol throws, where its
`String` said it couldn't. The `try` is the output's layout: the call is
caught where Rust catches it, and nothing else is.

## Context

`js::concat_text(value)` is JS's `value + ""` (ADR 0310), which throws of
a symbol and of an object whose `Symbol.toPrimitive` or `valueOf`
throws. Its result was a `String`, so React's `CheckStringCoercion`
couldn't be written:

```js
function willCoercionThrow(value) {
  try {
    testStringCoercion(value);
    return false;
  } catch (e) {
    return true;
  }
}

function testStringCoercion(value) {
  return '' + value;
}
```

A `String` can't be caught; what JS threw went past the program as a
panic does.

## Decision

**`concat_text`'s result follows its argument, by `ToText`'s associated
type, `T::Text`: of text and numbers, which can't throw, a `String`; of a
value of any shape, `&Unknown` and `Option<&Unknown>`, `Result<String,
&'static JsError>`, a JS call that may throw (ADR 0035).** One function,
whose JS is `value + ""` either way; with a `Result`, its call is `$try(()
=> value + "")`, as `json::stringify_with`'s `T::Stringified` says which
values may throw (ADR 0588).

```rust
pub fn will_coercion_throw(value: Option<&Unknown>) -> bool {
    match js::concat_text(value) {
        Ok(_) => false,
        Err(_) => true,
    }
}

js::concat_text(width) + "px" // a String, of an f64
```

**Whether a JS call threw, returned, is `try { f(); return false; } catch
{ return true; }`**: a `match` of its `Result` whose arms are `false` and
`true`, which MIR makes a test of the `Result`'s variant, and its
`is_err()` and `is_ok()`, `$try(() => f()).TAG === "Err"`. Preparation's
`try_catches` writes it (`threw`), where ADR 0637's `try` is written:

```js
export function willCoercionThrow(value) {
  try {
    value + "";
    return false;
  } catch {
    return true;
  }
}
```

## Why

- **The type says it**, as every binding that may throw says it: rustc
  makes the caller handle the `Err` of a value that may be a symbol, and
  leaves the `String` of a number as it was.
- **It's React's `try`**, the call alone in it, as Rust catches it alone.

## Alternatives

- **A second function, `try_concat_text`**: two spellings of `value +
  ""`, and the one of a `String` still wrong of a symbol.
- **A `Result` of every value**: a number's `+ "px"` would need an
  `unwrap()` of what can't fail.
- **The returned test kept, `return $try(() => value + "").TAG !==
  "Ok";`**: right, but a helper and its tag where React writes `try`.

## Consequences

- A program that took `concat_text` of an `Unknown` as a `String`
  handles its `Result`; its `unwrap()` is `$unwrapOk($try(() => value +
  ""))`, which throws Rust's panic of what JS threw. The bindings test's
  `formatStr` does, where react.dev's Console's `arg + ''` lets a
  symbol's throw go.
- The guide's `willCoercionThrow` and `hasConfig`.

## What holds it

- `test/bindings.test.ts`, "concat_text of any value is a Result, which a
  match catches": React's `willCoercionThrow`, `is_err()` and `is_ok()`,
  each run of a symbol, of an object whose `valueOf` throws, a `BigInt`
  and `None`; and a number's text, a `String`.
- Mutations in `scripts/mutations/prepare.ts`: the test not made a `try`,
  and its `true` and `false` the other way round.

## Amendment: `"" + value`, empty text first

`concat_text(value)` is `"" + value`, as React writes `'' + value`, where
it was `value + ""`; its link form is `"\"\" +"`, a prefix as `typeof` and
`!!` are, where it was `"+ \"\""`. Case A: nothing the program can observe.
`+` asks each side's primitive with no hint, `ToPrimitive(value,
default)`, its `Symbol.toPrimitive` or `valueOf` then `toString`, either
way round, and joins; a symbol throws the same `TypeError` on either side.
Only which operand is read first differs, and `""` is a literal.

Text joined to it still makes text of it: `"" + n + "px"` is `n + "px"`,
and `"#" + ("" + id)` is `"#" + id`. React's `willCoercionThrow` is `try {
"" + value; return false; } catch { return true; }`.
