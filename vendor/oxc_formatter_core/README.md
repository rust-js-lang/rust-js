# oxc_formatter_core, with #1269's fix

oxc's `crates/oxc_formatter_core` at the tag rust-js's oxc crates are
pinned to, `crates_v0.151.0` (bec650f15b457f3bffb6d79b65bf74bfcbaecdce),
MIT-licensed (`LICENSE`), with one change: its `will_break` of a slice of
elements (`src/format_element/document.rs`) asks an `Interned` element once.
Upstream, an `Interned` that won't break fell through to the next arm, which
asked it again, so each element interned inside another was walked twice:
calls nested 32 deep, typenum's dictionary of a type 32 levels deep, took
about 30 minutes to lay out, the time doubling with each level (#1269, ADR
1121). The answer is the same; only the second walk is gone.

Cargo's `[patch]` in the root `Cargo.toml` puts it in place of oxc's own.
Its manifest is written for a crate outside oxc's workspace: the same
dependencies, at the same tag. `test/architecture.test.ts` holds its tag to
the root's, so moving oxc's tag fails until this copy is taken again or,
once oxc has the fix, removed with its `[patch]`.
