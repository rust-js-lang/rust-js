# 0563. What's found once of a crate is keyed by the crate's own files

Status: Accepted.

Case: N ([0262](0262-when-rust-and-js-disagree.md)): nothing a program
can observe changes; only how long a compile takes.

## Context

Whether a field or a function gets a React name, a component's props or
a hook's `useX`, is found once for a crate, `props_structs` and `hooks`
in `src/lower/bindings.rs`, and kept in a static, `of_crate`, so the
playground, which compiles its crate again and again in one process,
finds it again when the crate is the same and anew when it changed. The
key was the crate's identity and its content: another crate's, its
metadata's hash; this one's, the hash of every file in rustc's source
map, each Debug-formatted and the strings concatenated, since rustc
hashes HIR only for a build that writes metadata.

The key is built on every lookup, each field and each function named
while a body is lowered. The source map holds not only the crate's own
files but each file of std a span has touched, and it grows as lowering
touches more: 220 files at the first lookup, 390 by the thousandth.
`adapter_drops.rs`, 137 lines, looked up a thousand times; `rc_counted_uses.rs`
sixteen hundred. That was half a million Debug formats of a 20-byte
array and as many string concatenations per compile, to find an answer
already kept. Profiled, it was 54% of one compile's self time and 74% of
another's; rustc's own passes were 20 ms of a 500 ms compile.

## Decision

The key of this crate's content is its own files' hashes, the
`SourceFileHash` of each file of the source map whose `cnum` is
`LOCAL_CRATE`, compared as values, not as text; another crate's is its
`Svh`. `Content` names the two.

Only the crate's own files, because that's what the answer is of: a
props struct is one of this crate's, a hook one of this crate's
functions, and a dependency's content is its own key's. A std file in the
source map says that a span pointed into it, not that this crate changed.

## Why

Measured alone, three runs each, before and after:

| Case | Before | After |
|---|---|---|
| `rc_counted_uses.rs` | 0.62 s | 0.19 s |
| `adapter_drops.rs` | 0.51 s | 0.23 s |
| `interpreter.rs` | 0.43 s | 0.17 s |
| `matrix_integers.rs` | 0.72 s | 0.52 s |
| `range_searches.rs` | 0.11 s | 0.08 s |

What's left of `matrix_integers.rs` is its 51 KB of JS being formatted.

## Alternatives

- **`tcx.crate_hash(LOCAL_CRATE)` for this crate too.** rustc computes it
  only for a build that needs it, one writing metadata or incremental,
  and a program compiled alone doesn't.
- **Key by the session**, a `TyCtxt`'s address or the source map's. An
  address is reused once freed, so the playground's next crate could be
  found as its last.
- **Find it once per compile instead of once per process**, in the
  crate's analysis. Right too, and the bigger change: `is_props` is asked
  by name mapping, which has only the `TyCtxt`. The key as it is now
  costs one file's hash per lookup, so the cache stays.
- **A timing test, and a mutation it catches.** A mutation's tests must
  run the compiler ([0093](0093-mutations.md)), so only a compile's time
  could catch the old key put back. The old key costs about 100 µs a
  lookup: a program of a thousand field reads compiles in 0.92 s with it
  and 0.81 s without, and one of four thousand in 12 s either way, its
  own lowering dwarfing the key. No bound tells those apart on a loaded
  machine. The rule is held by reading the key instead, and the cases
  above are what proved it.

## Consequences

- [architecture.test.ts](../../test/architecture.test.ts), "what's found
  once of a crate is keyed by the crate's own files", holds that `of_crate`
  reads the source map's files only filtered to this crate's, and compares
  no text.
- No mutation: none that only a compile's time could catch is worth a
  flaky test, as above.
- A compile of a program with many names looked up is two to three times
  faster; the corpus's median case, bound by its fixed costs, gains little.
