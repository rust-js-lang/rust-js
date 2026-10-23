# 0634. A registered symbol is a `const`, and a `match` of them a `switch`

Status: Accepted. Amends [0619](0619-a-primitive-thread-local-is-a-constant.md),
whose symbols of the registry are `const`s now, and
[0629](0629-a-match-of-constants-is-a-switch.md), whose constants are a
`Some`'s too and the registry's symbols. Extends [0590](0590-a-match-of-constants-names-them.md)'s
names and [0283](0283-js-crate-covers-typescripts-es-library.md)'s
`Symbol`.

Case ([0262](0262-when-rust-and-js-disagree.md)), each below: N, the
program can't tell, of a symbol's `===` and of a read only tested
against the registry's symbols; C, Rust's, paid for in output, of a read
whose `Some` the program tells, `$symbolFor(value)`.

## Context

React tags its elements and types with the registry's symbols and
switches on them everywhere:

```js
// shared/ReactSymbols.js
export const REACT_MEMO_TYPE = Symbol.for('react.memo');
// shared/getComponentNameFromType.js
switch (type) {
  case REACT_FRAGMENT_TYPE: return 'Fragment';
  case REACT_PROFILER_TYPE: return 'Profiler';
}
switch (type.$$typeof) {
  case REACT_PORTAL_TYPE: return 'Portal';
  case REACT_MEMO_TYPE: ..
}
```

The port declared each as a `thread_local!` of `&'static js::Symbol`,
`js::symbol::r#for("react.memo")` (ADR 0619 named it as a constant),
which a `match` can't name, so a port could only write `if ty ==
REACT_FRAGMENT_TYPE.with(..) { .. } else if ..`. The user decided
(2026-10-23) a registered symbol is a Rust `const`, a `match` of them
legal, idiomatic Rust, its JS upstream's `switch`.

## Decision

**`js::SymbolFor(key)` is the registry's symbol of `key`**, a value of
its key, `#[derive(Clone, Copy, PartialEq, Eq)] pub struct
SymbolFor<'a>(pub &'a str)`, so a `const` holds one and a `match` names
it as Rust's structural match does. Its JS is the symbol: made,
`Symbol.for(key)`, a `const`'s too, `export const REACT_MEMO_TYPE =
Symbol.for("react.memo");`, named as a `const` is (ADR 0031), TypeScript's
`symbol`; its key, `.0`, `Symbol.keyFor(symbol)`; `==` of two, `===`
(`js_eq`). A test of its key against text is one of the symbol, `symbol
=== Symbol.for(text)`, the `const` a pattern names it by where one does
(`constant_patterns`, `eq_value`), as that's how a pattern of one tests it
and it holds of any symbol. (Case N: a registered symbol is `===` to
another just where its key is equal.)

```rust
use js::{SymbolFor, Unknown};

pub const REACT_FRAGMENT_TYPE: SymbolFor = SymbolFor("react.fragment");
pub const REACT_PROFILER_TYPE: SymbolFor = SymbolFor("react.profiler");

pub fn name(ty: Option<&Unknown>) -> Option<&'static str> {
    match SymbolFor::from_value(ty) {
        Some(REACT_FRAGMENT_TYPE) => Some("Fragment"),
        Some(REACT_PROFILER_TYPE) => Some("Profiler"),
        _ => None,
    }
}
```
```js
export const REACT_FRAGMENT_TYPE = Symbol.for("react.fragment");
export const REACT_PROFILER_TYPE = Symbol.for("react.profiler");

export function name(ty) {
  switch (ty) {
    case REACT_FRAGMENT_TYPE:
      return "Fragment";
    case REACT_PROFILER_TYPE:
      return "Profiler";
    ..
  }
}
```

**`js::SymbolFor::from_value(value)` reads a value of any shape as one**,
an `Option<&Unknown>`, `js::get(ty, "$$typeof")`'s: `Some` of a symbol
the registry's, `None` of any other value.

- **Where the program only tests it against the registry's symbols, it's
  the value itself**: what a `match`, a `matches!` or an `if let`
  compares, its patterns each `_` or a `Some` of them, `Some(A | B)`,
  binding nothing (`body_queries::registered_reads`). Each such test is
  the value's `===` to the symbol, its `Some` taken as there
  (`OptionWrites::registered`): `switch (type.$$typeof) { case
  REACT_MEMO_TYPE: .. }`, `type.$$typeof === REACT_LAZY_TYPE`. (Case N:
  no value but the registry's symbol of a key is `===` to it, so no
  other, a symbol of its own, a string or none, matches a case, as none
  is its `Some`; `_` takes what's left, both ways.)
- **Elsewhere it's checked, `$symbolFor(value)`**: the value where it's
  the registry's, `undefined` where not, so its `None` is JS's
  `undefined`, ADR 0030's, where the program tells it, `None =>`, a
  `Some(symbol)` it binds, `.map(..)`. (Case C: Rust's, as a string or a
  symbol of its own would else be its `Some`.)

**A `Some` of constants, where it's the value itself (ADR 0030), is
those constants** (`hints::constants_of`), each case of ADR 0629's
`switch`: a `match` of `Some("s")`, `Some("d") | Some("i")` and
`Some("f")` is `switch (flag) { case "s": .. }`, as a `match` of the
strings is.

**`js::symbol::r#for` is gone**: `js::SymbolFor(key)` is the one way to
make the registry's symbol, of a key known or made, `SymbolFor(&key)`, a
`const`'s or a value's. A call of it is an error that says so, `js::symbol::r#for`
is gone: the registry's symbol of a key is `js::SymbolFor(key)`. `&js::Symbol`
is any other symbol, one of its own, `js::symbol::new("id")`, or a
well-known one, `js::symbol::iterator`, which a thread-local holds, named
as a constant (ADR 0619).

## Why

- **It's React's**: a `const` of each tag, a `switch` on the tag, one
  case each, `===` each test.
- **Idiomatic Rust**: a `const` and a `match`, Rust's structural match of
  a `PartialEq, Eq` type, nothing rust-js's but the type, and the read of
  a value of any shape, as `js::classify` reads one.
- **One way to make one**: a registered symbol is a value of its key, a
  `const` where its key's known, which `&js::Symbol` can't be; so
  `r#for`, a second way, is gone. A symbol of its own, unlike any other,
  isn't a value of a key, so it stays `&js::Symbol`.
- **The value itself where it can't be told apart**, so a port's `switch`
  reads as upstream's, and Rust's `None` kept where it can.

## Alternatives

- **A thread-local of `&js::Symbol`, compared by `==`**: what ports
  wrote, an `if` chain of `.with(..)`s, and no `match`.
- **`from_value` always checked**: `switch ($symbolFor(type))`, a helper
  upstream hasn't, where no case could tell.
- **`from_value` always the value itself**: `None` of a string would be
  its `Some`, `is_some()` true of `"react.memo"`, and `None =>` never
  taken.
- **A `SymbolFor` of `&'static str` alone, `r#for` kept for a key made**:
  two ways to make one; Flight's `Symbol.for(name)` is
  `SymbolFor(name)`.

## Consequences

- A `match` of an `Option`'s constants is a `switch` where it's three
  cases or more: `test/lowering-matches.test.ts`'s strings.
- `Some(C)` of a variable of an `Option`, not a read's, keeps its
  `Some`'s test around the `switch`, `if (ty != null) { switch (ty) { ..
  } }`, where `C` is a `const`, whose value tidy can't see isn't `undefined`;
  a literal's, `Some("s")`, it drops.
- `==` of a read to `Some(C)`, `SymbolFor::from_value(ty) ==
  Some(REACT_MEMO_TYPE)`, is checked, `$symbolFor(ty) == ..`, and names
  the constant's value: a `matches!` is the comparison that reads as
  upstream's.
- The js crate's coverage of `Symbol.for` is `SymbolFor`'s
  (`builtins/coverage.ts`'s `MADE_BY`).

## What holds it

- `test/lowering-symbols.test.ts`: "a match of the registry's symbols is
  a switch of the value read", "the registry's symbol is Symbol.for its
  key, and a read of one it tells is checked", "js::symbol::r#for is
  gone, an error naming js::SymbolFor".
- `test/lowering-matches.test.ts`, "a `match` of an `Option`'s strings
  from MIR ..": a `Some`'s constants are a `switch`.
- Mutations in `scripts/mutations/lower/body_queries.ts`,
  `scripts/mutations/lower/mir/hints.ts`,
  `scripts/mutations/lower/representation.ts`,
  `scripts/mutations/lower/std_impls.ts`,
  `scripts/mutations/lower/aggregates.ts` and
  `scripts/mutations/lower/mir/calls.ts`, `scripts/mutations/lower/mir/rvalues.ts`, `scripts/mutations/lower/mir/patterns.ts`.

## Amendment: a registered symbol and its key do nothing (2026-10-23)

`Symbol.for(key)` and `Symbol.keyFor(symbol)` have no effects
(`Expr::has_effects`): a `js::SymbolFor`'s key is read only of a value its
type says is a registered symbol, which `keyFor` can't throw of. So what
nothing reads of one goes, as a guarded arm's key MIR reads for its guard
did not, `Symbol.keyFor(ty);` twice in the arm. Found porting React's
`getComponentNameFromType`. `test/lowering-symbols.test.ts`, "a guarded
arm of a registered symbol reads no key it doesn't use"; mutation
`symbol-key-has-effects`.

