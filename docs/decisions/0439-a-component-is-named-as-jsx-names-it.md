# 0439. A component is named as JSX names it, and needs no `#[allow]`

Status: Accepted. Amends [0041](0041-react.md); extends
[0435](0435-rust-and-react-two-worlds.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)): a lint's warning
changes nothing a program does; and "Idiomatic Rust in" (AGENTS.md).

## Context

A component is a function named as JSX names it, `fn SiteHeader()`:
capitalized, as JSX tells `<SiteHeader>` from `<div>`, and as rust-js
tells a component from a function (`is_component`). rustc's
`non_snake_case` lint flags each, so every crate with components began
`#![allow(non_snake_case)]` (ADR 0041), which "Idiomatic Rust in" forbids:
no `#[allow]` for rust-js's sake.

Writing components snake case, `fn site_header()` named `SiteHeader` in
JS, was weighed: a component would then need another mark than its name,
its signature (a helper `fn row(..) -> JSX::Element` silently renamed
`Row`) or an attribute to learn, and the tag `<SiteHeader>` would name a
function that's `site_header`. Leptos, Dioxus and Yew write components
capitalized too, `#[component] fn App()`, their macro allowing the lint.

## Decision

**A component keeps the name JSX gives it, and rust-js allows the lint
for it**: the `jsx!` pass that makes a component's macro gives its
function `#[allow(non_snake_case)]`, as it gives a function only
`js::export_default!` names `#[allow(dead_code)]` (ADR 0222). No crate
writes the `#![allow]`; the examples, the playground and the READMEs
don't.

- **Only a component**: a capitalized function that isn't one,
  `fn NotComponent() -> u32`, is still flagged.
- **In the editor too**, where rust-analyzer checks through rust-js
  (`rust-js-check`, ADR 0222), and in `rust-js build`. A plain `cargo
  check`, which doesn't run rust-js's pass, still warns of each component,
  as it warns of what's used only in JSX, whose `jsx!` it doesn't expand:
  a created app's test accepts it with that noise.

## Alternatives

- **`[lints.rust] non_snake_case = "allow"` in a created app's
  `Cargo.toml`**: quiet in a plain `cargo check` too, but for every
  function, so `fn NotComponent()` too, under rust-js as well.
- **Snake-case components named `SiteHeader` in JS**: weighed above.

## Why

- **One name in each world, the same**: the function, its tag and its JS
  name are `SiteHeader`, guessable from one example.
- **No line to learn**: the `#![allow]` was something only rust-js users
  wrote. `test/jsx.test.ts` compiles components with none and holds that
  rustc warns of none of them, and of the function that isn't one.
