# @rust-js/std

std's own bodies, compiled to JavaScript by rust-js from the MIR the
toolchain ships, which the JS rust-js writes imports by the package's
name, `@rust-js/std/iter.js`, as it imports `@rust-js/runtime`'s helpers
([ADR 0561](../docs/decisions/0561-std-bodies.md)). A `Peekable`'s
`peekable`, `next`, `next_back`, `rfold`, clone and `Debug` are here, its
struct made the JS iterator it is where `peekable` makes it; its `peek`
and `next_if`, whose bodies reach memory or read as `next`'s, are helpers
of the runtime's over the same struct.

Generated: `bun run std` compiles [`lib.rs`](lib.rs), a crate of nothing
but the modules std's bodies are written into, with `--std-bodies`, and
writes each module's JS and the manifest beside it, which the compiler
builds in. `test/std-package.test.ts` holds them equal to what the
compiler writes. Released with the compiler, at its version.
