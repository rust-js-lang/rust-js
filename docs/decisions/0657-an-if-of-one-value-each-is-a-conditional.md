# 0657. An `if` of one value each, a function's too, is a conditional

Status: Accepted. Amends [0601](0601-a-module-variable-an-if-makes-is-a-let.md),
whose module variable an `if` makes was a `let` each branch set where a
branch was a function, and the tidy rule of a conditional of two values,
which took no function as a branch. Amended by
[0658](0658-a-comment-before-a-branchs-value-is-before-it-in-the-conditional.md):
a branch with a comment is a conditional too, its comment before its value;
and by [0870](0870-a-let-chains-value-reads-its-binding-first.md): a let
chain's value too, its binding read first.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: the same value,
written as a person writes it.

## Context

React's `reportGlobalError` is one of two functions:

```js
const reportGlobalError: (error: mixed) => void =
  typeof reportError === 'function' ? reportError : error => { .. };
```

Its port, a `thread_local!` of an `if`, was a `let` each branch set:

```js
let reportGlobalError;
if (typeof reportError !== "undefined") {
  reportGlobalError = reportError;
} else {
  reportGlobalError = (error) => { .. };
}
```

A thread-local's initializer is a body lowered and tidied as a function's
is, and the one rule that writes `if (c) { x = a; } else { x = b; }` as `x
= c ? a : b` (`tidy/branches.rs`, `small`) took no function as a branch,
"which reads best returned or assigned as it is". So a module's variable
and a function's `let` alike stayed an `if` of a function; one of two
numbers was `const NUMBER = c ? 1 : 2` already, of each.

## Decision

**A value an `if` gives, each branch one expression, is `c ? a : b`,
whatever the expression: a function, `(n) => { .. }`, is one.** The rule
is the one there was, so a module's variable, `const pick = c ? f : (n)
=> ..`, and a function's `let`, `const f = c ? .. : ..`, and its
`return`, are the same. Prettier lays a long one out, its branches each a
line, as React's is:

```js
const reportGlobalError =
  typeof reportError !== "undefined"
    ? reportError
    : (error) => {
        console.error(error);
      };
```

What keeps an `if`, as before:

- **A branch that does something before its value**, a statement, a
  `let`: `if (c) { log(); x = a; }`.
- **A chain of more branches**, `else if`, which reads best as it's
  written, and an element of more than two levels.
- **A branch with a comment**, which this rule has no place for yet.

## Why

- **React's JS**, and any JS person's: a function is one value, as a
  number is.
- **One rule for every value an `if` gives**: where it's a module's, a
  function's, or returned, it's `tidy`'s, which a thread-local's
  initializer is lowered by too.

## Alternatives

- **A size**, a branch of so many lines an `if`: what Prettier lays out
  is a layout's, not a statement's; and a statement before the value is
  what an expression can't hold.
- **Chains as nested conditionals, `a ? x : b ? y : z`**: they read as the
  program's `else if` does now, which ADR 0629's `switch` and the chain
  rule keep.

## What holds it

- `test/lowering-branches.test.ts`, "an if of one value each, a
  function's too, is a conditional": a thread-local of the host's function
  or a closure, a function's `let` of two closures, each a `const` of a
  conditional; a branch that logs first, and a chain, each an `if`; run.
- `test/bindings.test.ts`, "a binding of any as a value is a fn of a
  static Unknown": React's form, `const reportGlobalError = typeof
  reportError !== "undefined" ? reportError : (error) => ..`.
- Mutation `function-not-small`.

## Amendment: a let chain's value too (ADR 0870)

A let chain of one binding whose branches are each one value, `if a &&
let Some(x) = e { t } else { f }`, reads its binding first, `const x = a ?
e : undefined;`, and is then a conditional by this rule, `x != null ? t :
f`, as an `if let`'s is
([0870](0870-a-let-chains-value-reads-its-binding-first.md)).
