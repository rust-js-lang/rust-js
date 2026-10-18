# 0299. A return of `undefined` is a bare `return;`

Status: Accepted.

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A `None`, or `()`, returned early was `return undefined;`. JS gives
`undefined` for a bare `return;`, and that's how a person ends a function
with nothing to give: react.dev's NavigationBar ends its effect with
`} else { return; }`.

## Decision

**A `return` whose value is `undefined` is `return;`.**

## Why

- **It's the JavaScript a person writes.**
- **It's the same program**: `return;` gives `undefined`.

## Since

- **Preparation makes it**, before printing, as it makes the other
  rewrites of how JS is written; an arrow whose body is the value it
  gives keeps it, `() => undefined`. (Amended: the printer left the value
  out, a rewrite the architecture keeps from it, as it wrote `a || b` of
  `a ? a : b`, a test without its `!!`, and no JSX attribute of
  `undefined`, which preparation makes now too.)

## Amendment: a function's last one isn't written

**A function's last `return;`, a closure's too, isn't written**: the
function falls off its end there, as a person ends one, `function half(n)
{ if (n % 2 === 0) { return n / 2; } }`, and an `Iterator::next` that
always gives `None` is `function next() {}`. Decided with the user, over
`return undefined;` where another path gives a value, ESLint's
`consistent-return`, which would be a second form of one `None`. Case A:
falling off the end gives `undefined`, as `return;` does. Preparation
answers it, for every function body (`function_body`); the MIR lowering's
own drop of a last `return;` is gone, as preparation's covers it.
