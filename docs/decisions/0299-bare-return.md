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
