# @rust-js/std

Rust's std for JS: everything the JS rust-js writes imports, one package
with one entry, released with the compiler, at its version, as ReScript's
`@rescript/runtime` is ([ADR 0103](../docs/decisions/0103-runtime-package.md),
[ADR 0561](../docs/decisions/0561-std-bodies.md)).

```js
// frontend/src/form.js, as rust-js writes it
import { $debugStr, $parseInt, peekableIterator_next } from "@rust-js/std";
```

Its entry, `index.js`, re-exports two parts, which are its own:

- **The helpers**, `runtime.js`: bounds checks, Rust's integer arithmetic
  and formatting, iterators, JSON for serde, and the rest, each a `$`
  name, written by the compiler from the helpers it has
  (`rust-js --runtime-module`).
- **std's own bodies**, `iter.js`, compiled by rust-js from the MIR the
  toolchain ships, of [`lib.rs`](lib.rs), a crate of nothing but the modules
  they're written into, with `--std-bodies`. A `Peekable`'s `peekable`,
  `next`, `nth`, `last`, `next_back`, `rfold`, clone and `Debug` are here,
  its `peek` and `next_if` helpers over the same struct; a `Fuse`'s too,
  with both of std's `FuseImpl`s ([ADR 0566](../docs/decisions/0566-fuse-is-stds.md)).
  Its manifest, `manifest.json`, is built into the compiler.

Each is a named export and the package has no side effects, so a bundler
keeps only what a program uses. An app installs the compiler's own version:
the build adapter refuses another.

Generated: `bun run std` writes `runtime.js`, `iter.js`, its map, the
manifest and `index.js`; `test/std-package.test.ts` holds them equal to what
the compiler writes.
