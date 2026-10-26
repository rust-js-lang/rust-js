# 0657. An `if` of one value each, a function's too, is a conditional

Status: Accepted. Amends [0601](0601-a-module-variable-an-if-makes-is-a-let.md),
whose module variable an `if` makes was a `let` each branch set where a
branch was a function, and the tidy rule of a conditional of two values,
which took no function as a branch. Amended by
[0658](0658-a-comment-before-a-branchs-value-is-before-it-in-the-conditional.md):
a branch with a comment is a conditional too, its comment before its value;
and by [0870](0870-a-let-chains-value-reads-its-binding-first.md): a let
chain's value too, its binding read first. Amended: a branch that adds to
the variable it assigns keeps the `if`, as written.

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
- **A branch that adds to the variable it assigns**, `className +=
  " " + m`, as written: React's `if (className == null) { className =
  match; } else { className += ' ' + match; }`. (Amended: it kept an `if`
  only where both branches added to it.)

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

## Amendment: an `if` of `return`s the program wrote stays an `if`

React's getComponentNameFromFiber writes its `OffscreenComponent` case
`if (fiber.return !== null) { return getComponentNameFromFiber(fiber.return);
} return null;`. Its port, `let return_fiber = fiber.r#return.borrow().clone();
if let Some(r) = return_fiber { return get_component_name_from_fiber(&r); }
return None;`, printed `return returnFiber != null ?
getComponentNameFromFiber(returnFiber) : null;`: MIR joins the two
`return`s into one, each branch setting the function's value, which this
rule then made one value. **Where each branch's value is a `return` the
program wrote (`Jumps::returns`), the `if` is of `return`s, as written**:
`if (returnFiber != null) { return ..; } return null;`. An `if` the
program wrote as a value, `if let Some(p) = .. { p.depth } else { 0 }`,
is still `parent != null ? parent.depth : 0`, and a value MIR made of
branches no `return` gives, a `match`'s, a conditional. Case N: the same
branch runs and returns the same value.

The `const returnFiber = fiber.return;` the port names stays, read once,
as Rust reads it, by the name the program gave it; upstream reads
`fiber.return` twice, which the port's Rust doesn't. Reading it in place
was rejected: it drops the program's name and reads a place twice that
Rust reads once, a second form of one `let` picked by whether its value
is a property.

Held by `test/lowering-written-branches.test.ts`, "an if of returns the
program wrote stays an if", run in JS, and the mutation
`written-returns-made-a-conditional` (`tidy/branches.ts`).

## Amendment: a `return` of an `if` is its conditional returned

React's elementRefGetterWithDeprecationWarning writes `return refProp !==
undefined ? refProp : null;`; its port, `return if !js::strict_equals(ref_prop,
None) { ref_prop } else { None };`, printed `if (refProp !== undefined) {
return refProp ?? null; }` and the `else`'s `null` the function's last
`return null;`. MIR returns from each branch, as of `if c { return a; }
return b;`, and `values`' shared tails took the `else`'s `return null;` out
to the one after the `if` before `returned_values` made a conditional of
it, which it also left alone as `return`s the program wrote. **A `return`
the program wrote of an `if`, its value, is the conditional returned**:
`returned_values` takes a branch's `return` that's the program's `return`
of the `if` itself, as one it didn't write in the branch, and runs in the
`jumps` stage too, `jumps/returned ifs`, before a shared tail takes the
`else` apart. An `if` whose branches the program wrote as `return`s
stays an `if`, as above. Case N: the same branch's value is returned.
Held by `test/lowering-written-branches.test.ts`, "an if of returns the
program wrote stays an if", run in JS, and the mutation
`returned-if-a-statement` (`tidy/jumps/tails.ts`).

The port's own shape, the `return if` in `if __DEV__ { .. }` after a
warning given once and the function's tail `None` after it, still printed
`if (refProp !== undefined) { return refProp; }` and the tail's `return
null;`: there MIR doesn't return from each branch but assigns the
function's value in each, and `branch_values`' rule above, that an `if`
whose each branch's value is a written `return`'s stays an `if`, took
the `return` of the whole `if` for one in each branch. **A branch's
`return` is the program's only where the `if` isn't inside it**
(`written_returns`, given the `if`'s span): `return refProp !== undefined
? refProp : null;`. Held by the same test, its `ref_of` the port's shape,
run in JS with `__DEV__` set, and the mutation
`written-returns-of-a-returned-if` (`tidy/branches.ts`).
