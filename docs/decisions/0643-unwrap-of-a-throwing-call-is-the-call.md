# 0643. `unwrap()` of a throwing JS call is the call

Status: Accepted. Amends [0035](0035-results-and-throwing-js.md), whose
`unwrap()` of a throwing binding's `Result` was `$unwrapOk($try(..))`.

Case ([0262](0262-when-rust-and-js-disagree.md)): B, observable where it's
rare and harmless; its Consequences say where.

## Context

A binding that may throw gives `Result<T, &JsError>`, its call `$try(() =>
f(x))` (ADR 0035). A program that has no answer to the throw unwraps it,
as react.dev's Console does of `arg + ''`:

```rust
arg = Some(js::unknown_of(js::concat_text(arg).unwrap()));
```

and rust-js wrote `arg = $unwrapOk($try(() => "" + arg), undefined,
String)`: a helper that catches what JS threw to throw a panic of its
text, where react.dev writes `arg = arg + ''` and lets JS's own throw go.

## Decision

**`unwrap()` of a throwing JS call's `Result` is the call itself**, its
throw what JS threw: `json::parse(text).unwrap()` is `JSON.parse(text)`,
`js::concat_text(v).unwrap()` is `"" + v`, and `js::settle(p).await.unwrap()`
is `await p`, its rejection as it was. One rule, of every throwing call's
`Result`: what lowering made `$try(() => call)` or `await $settle(p)`,
unwrapped where it's made. `Expr::uncaught` answers what such a `Result` is
of where nothing catches it, beside `Expr::tried`, which makes it, and
`Expr::tried_call`, which preparation's `try` reads (ADRs 0637, 0639).

- **`expect(msg)` keeps its message**, `$unwrapOk($try(..), msg, ..)`:
  `msg` is the program's own words, why it's sure, and the bare call
  would drop them; the panic says `valid JSON: SyntaxError: ..`. Of
  `unwrap()`, Rust's text says nothing the program wrote.
- **A `Result` held, then unwrapped**, `let r = f(); ..; r.unwrap()`, is
  `$try`'s still: the call ran where it was made, and its throw is caught
  there, before what follows runs.

## Why

- **It's the JS a person writes**: react.dev's `arg + ''`, `JSON.parse(text)`,
  no helper.
- **It's a throw either way**, where Rust's is a panic: the same code
  after it doesn't run, the same `finally` drops run as it unwinds (ADR
  0437).

## Alternatives

- **`$unwrapOk($try(..))`, as before**: Rust's message, at the cost of a
  `try`, a helper and a second error object around JS's.
- **`expect(msg)` the bare call too**: one form for both, but its message,
  which the program wrote to be read, gone.

## Consequences

- **Where it's observable**: only JS that catches the throw, of a
  `pub` function a JS caller calls, sees JS's own error, a `SyntaxError`,
  where Rust's panic is an `Error` whose message is ``called
  `Result::unwrap()` on an `Err` value: SyntaxError: ..``. Rust code can't
  catch either (`catch_unwind` is refused), and an uncaught one is the
  host's error either way. Listed in the guide's semantics.
- The bindings test's `formatStr` is `arg = "" + arg;`, react.dev's.

## What holds it

- `test/js-builtins.test.ts`, "unwrap of a throwing JS call is the call":
  `JSON.parse(text)`, `"" + value` and `await p`, each run where it
  throws, and rejects, with JS's own error; `expect`'s message kept; a
  `Result` held, `$try`'s.
- Mutations in `scripts/mutations/lower/std_types/option.ts` and
  `scripts/mutations/js/make.ts`: the call caught, `expect`'s message
  dropped, `$settle` kept.
