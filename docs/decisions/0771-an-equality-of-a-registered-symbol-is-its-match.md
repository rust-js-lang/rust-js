# 0771. An `==` of a value read as the registry's symbol is its `matches!`

Status: Accepted. Amends [0634](0634-a-registered-symbol-is-a-const.md),
whose value read as the registry's symbol was itself only where a
`match`, a `matches!` or an `if let` tested it.

Case ([0262](0262-when-rust-and-js-disagree.md)): A: the same answer for
every value, by `===`; only how the JS reads changes.

## Context

React's port tests an element's type two ways, both Rust a person
writes:

```rust
matches!(SymbolFor::from_value(x), Some(REACT_MEMO_TYPE))
SymbolFor::from_value(x) == Some(REACT_MEMO_TYPE)
```

The first was `x === REACT_MEMO_TYPE`; the second
`$symbolFor(x) == Symbol.for('react.memo')`: a helper, the constant's
value where its name was written, and `==`. Two outputs for one
comparison. The `Some(REACT_MEMO_TYPE)` of an `==` is a promoted
constant, `&Some(C)`, which rustc evaluates, so its name was gone.

## Decision

**`from_value(x) == Some(s)`, and `!=`, either way round, is
`matches!(from_value(x), Some(s))`'s form: `x === s`, `x !== s`.** The
read is the value itself, as `registered_reads` says of a `match`'s, and
the comparison is `===` whatever `s` is: a symbol of the registry's is
only `===` to itself, and `==` would find an object that turns into one
equal to it.

**A promoted `&Some(C)` of a named `const` rust-js compiled is `Some` of
`C` by its name** (ADR 0031), and an `Option` it's compared with by `==`
is compared by `===`: it's never `None` (`made_some`), as a `Some` made
in a variable isn't, through a reborrow too.

```rust
pub fn eq(x: Option<&Unknown>) -> bool {
    SymbolFor::from_value(x) == Some(REACT_MEMO_TYPE)
}

pub fn kept(x: Option<&Unknown>) -> Option<SymbolFor<'static>> {
    let s = SymbolFor::from_value(x);
    if s == Some(REACT_MEMO_TYPE) {
        return None;
    }
    s
}
```

```js
export function eq(x) {
  return x === REACT_MEMO_TYPE;
}

export function kept(x) {
  const s = $symbolFor(x);
  if (s === REACT_MEMO_TYPE) {
    return;
  }
  return s;
}
```

A read kept in a variable is still checked, `$symbolFor(x)`: it's given
back, where its `None` is `undefined`.

## Why

- **One comparison, one output**: `==` and `matches!` of a `Some` say the
  same, and are written the same.
- **`===` is the right answer**: `from_value(x) == Some(s)` is true of `x`
  the symbol `s` alone.

## Alternatives

- **The `==` as it was, `$symbolFor(x) == s`**: right, but a helper and
  a loose `==` React's `x.$$typeof === REACT_MEMO_TYPE` has neither of.
- **Only a named `Some(C)`**: `Some(SymbolFor("k"))` would be another
  form; the read is the value whatever the `Some` holds.

## What holds it

- `test/lowering-symbols.test.ts`, "an == of a value read as the
  registry's symbol and a Some is the matches! form": `==`, `!=`,
  flipped, a literal `Some`, and a read kept, run on symbols, strings,
  `undefined` and an object whose `Symbol.toPrimitive` gives the symbol.
- Mutations `registered-equality-unread` (`body_queries.rs`),
  `registered-equality-loose` (`mir/dispatch.rs`),
  `promoted-some-unnamed` (`mir/operands.rs`), `promoted-some-maybe-none`
  and `reborrowed-some-maybe-none` (`mir/patterns.rs`).

## Amendment: a `let` of the read that's only compared (2026-10-28)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, as the decision's:
no value but the symbol is `===` it.

React's mountLazyComponent, `const $$typeof = Component.$$typeof; if
($$typeof === REACT_FORWARD_REF_TYPE) { .. } else if ($$typeof ===
REACT_MEMO_TYPE) { .. }`, its port's `let r#typeof =
SymbolFor::from_value(js::get(component, "$$typeof")); if r#typeof ==
Some(REACT_FORWARD_REF_TYPE) { .. }` (gap ml1 of the port), checked the
read where the `let` held it, `const typeof$ =
$symbolFor(component.$$typeof)`.

**A `let` of a `from_value` read, every read of which is an `==` or `!=`
of it and a `Some(..)`, is the value** (`body_queries::registered_lets`):
its read is a registered read, and each comparison a registered equality,
`const typeof$ = component.$$typeof; if (typeof$ ===
REACT_FORWARD_REF_TYPE)`. One read any other way, returned or given on,
is checked as before.

Held by `test/lowering-symbols.test.ts`, "an == of a value read as the
registry's symbol and a Some is the matches! form", its `tagged`, run;
mutations `registered-let-compared-checked` and
`registered-let-of-any-read` (`body_queries.ts`).

