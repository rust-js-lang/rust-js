# 0529. A shipped crate is qualified, and memchr's searches are JS's

Status: Accepted. Extends [0415](0415-a-crate-of-crates-io-in-an-app.md)
and [0423](0423-hashbrowns-maps-are-stds.md); accepts the shipped-crate
contract and its first crate, memchr, of the drafts
`shipped-crates.md` and `shipped-memchr.md`.

Case: C ([0262](0262-when-rust-and-js-disagree.md)) for indices, search
order and what an iterator gives; A for replacing memchr's SIMD and
pointer engine with byte comparisons, which no program can observe; D for
memchr's API outside what's qualified, and for a memchr that isn't.

## Context

memchr finds a byte in a slice with pointer arithmetic and SIMD, which
rust-js refuses to compile. Many crates use it, regex among them. A crate
rustc builds, as hashbrown is (ADR 0423), whose API rust-js writes the JS
of, needs more than a name to stand on: a fork or a path dependency named
`memchr` isn't memchr, and the API rust-js writes is one version's.

## Decision

**One catalog says which crates rustc builds, and of one whose API rust-js
writes, the versions of crates.io's it's qualified at: memchr 2.8.3.
Anything else by that name, another version, a fork, a path or a git
dependency, is its types alone, refused where it's called, saying which
memchr to depend on.**

- **Cargo owns the catalog**, `cargo::READ_AS_RUSTC_BUILDS_THEM`, the one
  list, each entry its name and, where rust-js writes its API, the
  versions it's qualified at. A qualified crate is crates.io's: its
  directory is the registry's.
- **An unqualified one** takes the path a crate rust-js couldn't compile
  takes (ADR 0415): rustc checks its types, and why is written beside its
  metadata, which a call into it says: "memchr 2.8.3 from /path, not
  crates.io, isn't the memchr rust-js writes the JS of, crates.io's 2.8.3:
  depend on it, `cargo update -p memchr --precise 2.8.3`".
- **Recognition maps memchr's items once**, `memchr_op` and
  `is_memchr_iter`, of the memchr rustc built, not refused, nor a library
  rust-js compiled.

**memchr's searches are the runtime's, over the byte arrays slices already
are, no copy:**

```rust
memchr::memchr(b'o', text);
memchr::memrchr2(b'w', b'e', text);
let it = memchr::memchr2_iter(b'o', b'r', text);
```

```js
$memchr(text, [111]);
$memrchr(text, [119, 101]);
const it = $memchrIter(text, [111, 114]);
```

- `memchr`, `memchr2` and `memchr3` are `$memchr`, their `memrchr`s
  `$memrchr`, an index of the slice given, `undefined` for `None`; of one
  needle, JS's own `indexOf`, fifteen times faster than comparing each byte
  on a 1 MiB slice, nine times on 64 bytes.
- Their iterators are `$memchrIter`, which knows its back, the two ends
  meeting as memchr's do, and carries its clone, where it is (ADR 0525);
  `memrchr_iter` is its `rev()`, as memchr's is.
- Everything else of memchr, `memmem`'s finders, is refused where it's
  called, as hashbrown's own methods are.

## Why

- `test/shipped-crates.test.ts` runs `scripts/crate-corpus/memchr.rs`, the
  searches of one, two and three needles from either end, empty slices,
  every byte value, repeated and equal needles, a slice of a slice,
  adjacent matches, a generated array searched both ways, and iterators
  read from both ends, cloned partway and read after they end, against
  native memchr 2.8.3; and refuses `memmem::find`.
- `test/registry-crates.test.ts` refuses a git fork named memchr, at
  2.8.3, where it's called, saying why.

## Consequences

- serde's build (`serde/`) pins memchr 2.8.3, as serde_json's dependency
  already was, for a program to name.
- regex and url stay drafts, each to be qualified under this contract.
- Mutations accept a fork, lower a refused memchr's call, find forward
  where it's backward, step a `memrchr_iter` forward, look for the first
  needle alone, skip the first byte, pass the iterator's back, restart a
  clone, and refuse its back and its clone.
