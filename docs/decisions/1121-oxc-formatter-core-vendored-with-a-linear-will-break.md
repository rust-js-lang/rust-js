# 1121. oxc_formatter_core vendored, its `will_break` asking each interned element once

Status: Accepted (#1269, 2026-11-11; the workaround here decided through
m6-react, upstream's report left to the user).

Case ([0262](0262-when-rust-and-js-disagree.md)): none. No program's
behaviour changes, nor its JS: only how long laying it out takes.

## Context

rust-js lays out the JS it writes with oxc_formatter, oxfmt's layout (ADR
0381). The Crater (ADR 1109, #1174) met typenum's `uint::fmt_tests::binary`,
whose `Binary` dictionary of `U2147483648`, a type 32 levels deep, is a call
nested 32 deep: rust-js ran past the Crater's 20 minutes on it. Calls nested
`n` deep took time doubling with each level, in oxc_formatter alone, at its
default options:

| depth | 16 | 18 | 20 | 22 | 24 | 32 |
|---|---|---|---|---|---|---|
| before | 16 ms | 48 ms | 173 ms | 774 ms | 3,455 ms | about 30 min |
| after | 2 ms | 0 ms | 0 ms | 0 ms | 0 ms | 0 ms (64: 0 ms) |

Prettier lays out 26 deep in about 100 ms. The cause is one arm of
`oxc_formatter_core`'s `will_break` of a slice of elements
(`format_element/document.rs`):

```rust
FormatElement::Interned(interned) if ignore_depth == 0 && interned.will_break() => {
    return true;
}
// ..
element if ignore_depth == 0 && element.will_break() => {
    return true;
}
```

An interned element that won't break fails the first arm's guard and falls
to the last, whose `element.will_break()` of an `Interned` asks
`interned.will_break()` again. Each call's arguments are interned and hold
the inner call's, so every level walks what's inside it twice: 2^depth.

## Decision

**`vendor/oxc_formatter_core/` is oxc's crate at rust-js's pinned tag,
`crates_v0.151.0`, with that arm made the element's own**, Cargo's `[patch]`
putting it in place of oxc's in both compilers, the native one's
`Cargo.toml` and the browser's `wasm/Cargo.toml`:

```rust
FormatElement::Interned(interned) => {
    if ignore_depth == 0 && interned.will_break() {
        return true;
    }
}
```

The answer is the same in every case, as the arm it fell to asked the same
question; only the second walk is gone. Its manifest is written for a crate
outside oxc's workspace, the same dependencies at the same tag; its
`README.md` says what changed and why, and its `LICENSE` is oxc's MIT.

- **Not a fork of oxc** on a public host: publishing one is an action
  outside the repository, which is the user's to take, as reporting it
  upstream is.
- **Not leaving deep calls unformatted**, the formatter skipped past some
  depth: that JS would read otherwise than the rest, and the threshold would
  be a guess at what's slow.
- **Not a memo of `will_break`**: the walk once is the whole fix, and a memo
  would change more of oxc's code than the one arm.

## Consequences

- `src/format.rs`'s `nested_calls_laid_out_in_linear_time` lays out calls
  32 deep within 5 s, in a thread, so a regression fails it rather than
  hanging the suite; the corpus case `calls_nested_32_deep_laid_out.rs`
  compiles typenum's shape, a type-level number of 32 bits, and a call 32
  deep, against native Rust, within the corpus's compile limit. With the
  arm put back, both fail (the mutations cover `src/` alone, so this was
  shown by hand).
- typenum's 1,762 tests run as native in 17 s, its `cut: #1269` line taken
  off `scripts/crater-decided.txt`.
- `test/architecture.test.ts` holds the copy's oxc tag to the one every oxc
  crate is pinned to, and both manifests' `[patch]`: moving oxc's tag fails
  until the copy is taken again from the new tag or, once oxc has the fix,
  removed with its `[patch]`.
