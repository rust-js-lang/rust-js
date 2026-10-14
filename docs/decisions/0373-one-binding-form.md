# 0373. A binding of a JS function is an ordinary function, and a JS type a struct

Status: Accepted. Amends [0021](0021-js-interop.md),
[0024](0024-web-crate.md), [0028](0028-js-module-imports.md),
[0039](0039-generic-bindings.md) and [0111](0111-js-types-as-structs.md). Amended by [0376](0376-keyword-names-are-raw.md): a
parameter or a field named by a Rust keyword is its raw identifier,
`r#type`.

Case: N ([0262](0262-when-rust-and-js-disagree.md)): the JS a binding's
call writes is the same either way; only how it's declared changes.

## Context

A JS function had two bindings that did one job: a function in an
`extern` block, `#[link_name = "new Event"] pub safe fn new(..) -> ..;`, and
an ordinary function with `#[rust_js::link_name]` and an `unreachable!()`
body (ADR 0039). About 4,700 bindings were the first, 24,000 the second,
and the webapi, React, Node.js and builtins crates each mixed them: an
agent writing one had to choose, and only the second can be generic. A JS
type had two as well: an `extern` type, `type Url;`, of a nightly feature,
and a struct of a `JsObject` (ADR 0111), which kept the first "where a
nightly allows it". AGENTS.md's first rule is exactly one way to do each
job.

## Decision

- **A binding of a JS function is an ordinary function**, as ADR 0039 has
  it: `#[cfg_attr(rust_js, rust_js::link_name = "new Event")] pub fn
  new(r#type: &str) -> &'static Event { unreachable!() }`. One that was
  `unsafe` to call in an `extern` block, as every one not marked `safe`
  is, is an `unsafe fn`.
- **An `extern` block holds `static`s alone**, a JS global or an import,
  which have no body to write: `#[link_name = "./hero.png#default"] safe
  static hero_img: &'static str;`.
- **A JS type is a struct of a `JsObject`** (ADR 0111): an `extern` type is
  refused.
- **The other forms are compile errors that say the one**: a function in
  an `extern` block, declared in the crate or called from another, and an
  `extern` type, each refused with the form to write
  (`bindings::extern_fn_refused`, `validation.rs`).
- **The generators write the one form**: webapi's, the typed arrays', the
  Google fonts' and React's elements; the crate-level allowances of
  `clashing_extern_declarations` and `invalid_runtime_symbol_definitions`,
  which only `extern` functions needed, are gone.

## Consequences

- Every binding in the repository's crates, tests, examples and the
  playground is the one form; the JS each writes is as it was, the corpus
  and the examples' snapshots unchanged.
- `test/diagnostics.test.ts` holds both refusals.
