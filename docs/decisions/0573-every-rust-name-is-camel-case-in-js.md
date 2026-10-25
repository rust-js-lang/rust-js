# 0573. Every Rust name is camelCase in JS

Status: Accepted; extended by [0855](0855-a-parameters-rust-js-name-is-its-js-name.md),
whose parameter's `#[rust_js::name]` is its JS name; amended by [0678](0678-a-parameters-underscore-isnt-its-js-name.md),
whose parameter is without the `_` that marks it unused, and by [0579](0579-a-thread-local-is-named-as-a-variable.md),
which names a thread-local as its module's variable. Amends [0435](0435-rust-and-react-two-worlds.md), whose
"what's Rust's own keeps Rust's" it replaces; supersedes
[0563](0563-found-once-is-keyed-by-the-crates-own-files.md). Extends
[0038](0038-js-names-and-destructuring.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): A. Which name an item
has in JS changes nothing a program does, only who can guess it.

## Context

ADR 0435 gave React's things React's names and kept the rest Rust's: a
hook, a `use_x` that calls one, was `useX`, a component's props' fields
camelCase, and every other function, field and method kept its Rust name,
`first_word`, `sort_index`. Porting React itself (rust-js-lang/react-in-rust)
showed what that costs:

- **The JS reads as no JS person writes it.** React's scheduler is
  `node.sortIndex`, `peek(heap)`, `push(heap, node)`; ported, it was
  `node.sort_index`, half its names Rust's and half JS's, by a rule only
  rust-js has.
- **Which rule a name follows was a guess.** A field was camelCase if its
  struct was a component's props, found by which functions take it and
  which JSX spreads it; a function, if it was named `use_x` and called a
  hook, found by walking the crate's calls. A props struct used only in
  another crate's JSX kept its Rust names. Each answer was right and none
  could be guessed from the Rust.
- **The finding had a cost of its own**: a cache of each crate's answer,
  keyed by its files' hashes so the playground's crate found it again
  (ADR 0563).

## Decision

**Every name a Rust item has in JS is its `#[rust_js::name]`, else its
Rust name in camelCase**, `bindings::name_in_js`'s one rule: a function,
a field, a method, a variant's field, a trait's method, the crate's and
std's alike.

```rust
pub struct Node { pub id: u32, pub sort_index: u64 }
pub fn first_word(text: &str) -> &str { .. }
pub fn use_clicks() -> u32 { .. }
```

```js
export function firstWord(text) { .. }
export function useClicks() { .. }
node.sortIndex
```

- **A hook and a prop need no finding.** `use_clicks` is `useClicks`
  because every `snake_case` name is camelCase, and `on_click` is
  `onClick`; `is_hook`, `is_props` and their cache are gone.
- **A dictionary is keyed by the same rule.** A std trait's method in a
  dictionary std's impl makes, or the runtime's, is its JS name,
  `bindings::trait_method_name`: `{ asRef }`, `IIterator.sizeHint(it)`,
  `hasher.writeU32(n)`, as the crate's own impls' are. The runtime's
  stand-ins for std's types and traits are keyed so too: `$Hasher`'s
  `writeU32`, a `fmt::Write`'s `writeStr`, a `Utf8Error`'s `validUpTo`,
  a `SliceIndex`'s `indexMut`.
- **A binding to a JS name that is snake_case says so**, as any other
  name that isn't the camelCase one: Node's `process.config.variables`,
  `host_arch`, Next.js's metadata, `short_name`, a JWK's `key_ops`, each
  `#[rust_js::name]`. The binding generators write it where
  `scripts/rust-names.ts`'s `needsJsName` says, the one answer of the
  three they had: a Rust name not the JS one, or a JS name with a `_`. A
  raw identifier's, `r#type`'s `"type"`, which repeated its own, is gone.
- **What's printed stays Rust's.** `{:?}` of a `Utf8Error` is still
  `Utf8Error { valid_up_to: 1, error_len: None }`: a program sees the
  text, not the key.
- **One place makes a name camelCase**: `camel_case`, which a name's
  words, `snake_case`'s, are joined by, and `upper_first`, a word joined
  after the first. A module's default import, `langRust` of
  `@codemirror/lang-rust`, an impl's dictionary's crate when two clash,
  and a default import's class are each made by them, not by a loop of
  their own. serde's `rename_all = "camelCase"` is serde's own rule, which
  a program sees in its JSON, and stays serde's.
- **A name that isn't the camelCase one** says so with
  `#[rust_js::name]`, the one way to name an item in JS.

## Why

- **JS is read as JS is written.** A port reads as its original, and a JS
  caller of a Rust crate writes `firstWord`, as it writes every other
  library's names.
- **One rule, guessable from one example.** Having seen `sortIndex`, every
  name follows; nothing depends on who calls a function or spreads a
  struct.
- **Rust stays Rust.** The Rust is `snake_case` as rustc's lints want;
  the camelCase is JS's, made where JS is written.

## Alternatives

- **Keep ADR 0435's two worlds**: what this replaces, above.
- **A crate's switch to camelCase**, ADR 0046's `camel_case!`: one Rust
  giving two JS by a line, superseded by ADR 0435 already.
- **camelCase the crate's names, keep std's dictionaries Rust's**: two
  keys for one method, the crate's impl's `asRef` and std's `as_ref`, for
  one dictionary to hold. It broke 28 corpus cases. Rejected.

## Consequences

- Every generated module's names change: the snapshots, the corpus's JS,
  `@rust-js/runtime` and `@rust-js/std` are blessed with them.
- A JS caller of an existing crate's `first_word` writes `firstWord`.
- ADR 0563's cache, and its test, go with what it cached.
- A temporary rust-analyzer names after the call that made it (ADR 0569),
  `checkedTotal` of `checked_total(..)`, now has the called function's JS
  name, and is named `value` instead: the renames' share in
  `docs/readability.txt` grows from 825.5 to 838.1 in ten thousand lines,
  some 130 more `value$n` and `tmp$n`. Naming such a temporary well is the
  next readability work it leaves.

## What holds it

- `test/architecture.test.ts`, "a JS key the compiler or the runtime
  writes is camelCase": no `snake_case` literal is a JS key in lowering,
  nor a key or a method in a runtime helper.
- `test/architecture.test.ts`, "a snake_case name is made camelCase by
  camel_case alone": no other module splits a name on `_`.
- The corpus's cases of std traits' dictionaries, `generic_as_ref.rs`,
  `hash_values.rs`, `generic_writers.rs` and the rest, run in Node.

## Amendment: a name that repeats its own is an error (2026-10-22)

With every name camelCase, some 1,700 `#[rust_js::name]`s said what their
items were called anyway, `#[rust_js::name = "addEventListener"]` of
`add_event_listener`: a second way to write each name. They're gone, and
one is rust-js's error, of a program and of a crate `--rustc` compiles
for its metadata, webapi's, react's, next's alike:

```text
error: rust-js: `sort_index` is `sortIndex` in JS without `#[rust_js::name = "sortIndex"]`: remove it
```

- **What an item is called where it says nothing** is one function's,
  `bindings::default_name`: a function's, a `const`'s, a `static`'s or a
  field's camelCase; a variant's and a JS class's, its name. The readers
  and the check ask it alike. A unit struct's name is the string it is,
  which it has no other way to say, and is never a repeat.
- **The generators write a name only where it differs**:
  `scripts/rust-names.ts`'s `needsJsName`, of its `camelCase`, for
  webapi's, react's attributes', the builtins' Intl and Next.js's fonts.
  `camelCase` is the compiler's `camel_case` across the boundary,
  held equal to it by a test that compiles each of a set of names, and
  next's coverage reads a field's JS name by it.

What holds it: `test/bindings.test.ts`, "a generator's camelCase is the
compiler's, and a name that repeats it is an error".
