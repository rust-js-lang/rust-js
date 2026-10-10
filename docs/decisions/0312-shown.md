# 0312. `js::shown` is any value as a template shows it

Status: Accepted. Extends [0310](0310-any-value-as-js-reads-it.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev's Console keys what it logged by `${msg}-${index}`, of any value,
`null` too. A template's `${msg}` is JS's own text of it, which throws of
a symbol, where `String(msg)` names it, and Rust's `format!` takes only
what has a `Display`.

## Decision

**`js::shown(value)`, of any value `ToText` takes, displays as JS's
template shows it: `format!("{}-{index}", js::shown(msg))` is
`` `${msg}-${index}` ``.** Its `Display::fmt` is a binding of the value
itself, `#[link_name = "this"]`, and `format!` writes a value whose
`Display` is so as itself.

## Why

- **It's the JavaScript a person writes**: react.dev's template, exactly.
- **It's the same program**: the template makes the text, as the
  original's does, symbols and all.
- **It's tested**: a bindings test shows `undefined`, `null`, an object and
  text; a mutation calls its `Display`.

## Amendment: a number

`js::shown` takes an `f64` too: `format!("© {}", js::shown(year))` is
`` `© ${year}` ``, as react.dev's copyright shows `new Date().getFullYear()`.
Rust's `{}` of an `f64` stays `$displayF64`, as Rust writes it, `-0` and
`1000000000000000000000` where JS writes `0` and `1e+21`: `js::shown` is
how a program asks for JS's.
