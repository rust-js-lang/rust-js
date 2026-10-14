# 0376. A JS name that's a Rust keyword is its raw identifier

Status: Accepted. Amends [0024](0024-web-crate.md) and
[0373](0373-one-binding-form.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)): what a binding is
called in Rust; the JS it gives is the same name, `type`.

## Context

A JS name that's a Rust keyword, `type`, `use`, `for`, `ref`, was spelled
two ways in the bindings: webapi's generator wrote `type_` (283 times, and
`async_`, `loop_`, `ref_` and others), React's and `Intl`'s wrote `r#type`,
and the hand-written crates had `type_` with `#[rust_js::name = "type"]`
beside a field, `use_` for React's `use`, `for_` and `match_` of `Symbol`.
Each generator also had its own keyword list and its own word splitter.
AGENTS.md: exactly one way to do each job.

## Decision

**A JS name that's a Rust keyword is its raw identifier, `r#type`, in
every binding, generated or written by hand.** The four keywords Rust has
no raw identifier of, `self`, `Self`, `super` and `crate`, take a `_` after
them, `self_`, as nothing else can.

- **One naming module, `scripts/rust-names.ts`**: its `snake` names a JS
  name as a Rust function, field, module or parameter, `getElementById`
  `get_element_by_id`, `HTMLIFrameElement` `html_iframe_element`, a keyword
  escaped; `plain` is what it escaped, for a name made of more,
  `continue_with_key`. React's, webapi's and the builtins' generators ask
  it.
- **No `rust_js::name` for a keyword**: a raw identifier's name is the word,
  `type`, to rustc, rustdoc, serde and rust-js, which gives a field, a
  parameter or a property its JS name with nothing beside it. A raw
  identifier that's a JS reserved word, `r#yield`, is `yield$` where JS
  needs a binding's name, as any reserved word is.
- **Held**: the architecture test refuses a `keyword_` name a raw
  identifier could be in any crate's source, and a generator with its own
  keyword list or splitter; each generator's output is checked against what
  it writes, webapi's too.

## Why

- **It's Rust's own way**: `r#` is the language's escape for a keyword, which
  the tools and serde already read as the word. `type_` is a convention of
  `web-sys`, not of Rust, and a second name beside the JS one.
- **It's one rule, mechanical and reversible**: a keyword gets `r#`, and the
  JS name is the word. `type_` needed a `rust_js::name` on every field, a
  second spelling, and could collide with a JS name `type_`.
- **JS is the same, and reads better**: `{ type: "a" }` either way, and a
  parameter `r#type` is `type` in JS, where `type_` was `type_`.

## Consequences

- A program that called `.type_()`, `react::use_`, `symbol::for_`,
  `process::ref_` or named `type_:` in an options struct writes `r#type`,
  `react::r#use`, `symbol::r#for`, `process::r#ref`, `r#type:`; rustc's
  error is where, and its name is the JS one.
- React's `default` attribute is `default`, which isn't a keyword:
  `r#default` was the same identifier, and still compiles.
