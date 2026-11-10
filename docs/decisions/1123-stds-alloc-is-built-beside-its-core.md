# 1123. std's `alloc` is built beside its `core`

Status: Accepted. Extends [0561](0561-std-bodies.md)'s
`@rust-js/std` and #424's sysroot built to unwind (`src/std_sysroot.rs`).

Case: none of [0262](0262-when-rust-and-js-disagree.md)'s: what's built
for `@rust-js/std` to be lowered of, no change to what any program does.

## Context

`@rust-js/std` is std's own bodies, each lowered from the MIR of a `core`
built to unwind (#424): `bun run std` compiles `std/lib.rs` with
`--std-bodies`, whose session is given a sysroot of that `core` and a
`compiler_builtins` that's a crate and nothing more. Every compiled body
so far is `core`'s, the iterator adapters and `Iterator`'s provided
methods.

`io::Read`, `BufRead` and `Seek` (#1156) need std's bodies of `alloc`:
in the pinned toolchain `Read`, `BufRead`, `Cursor`'s impls, `&[u8]`'s and
`VecDeque<u8>`'s, `empty`, `repeat`, `Chain`, `Take`, `Bytes` and
`BufReader` are `alloc::io`'s. The provided methods #1156's
decision (a) compiles of std's bodies, `default_read_exact` and `BufRead`'s
`read_until`, `read_line`, `lines` and `split`, are among them. The
sysroot had no `alloc`, so `--std-bodies` couldn't name them.

## Decision

**The sysroot `--std-bodies` is given builds `alloc` too, of the
toolchain's own `rust-src`, after `core` and `compiler_builtins`, as they
are: metadata alone, each body's MIR kept, built to unwind.** It's keyed
as they are, by the toolchain, the target and each crate's arguments and
settings, so a sysroot made before has another key and is made again
once.

Nothing is linked, and a program's own session is unchanged: it reads the
toolchain's sysroot, whose `alloc` keys std's items by the same paths
(`lower::library::key`, crate name and def path). Which bodies of `alloc`
are compiled is the catalog's, `recognition::std_items`, as `core`'s are;
this decision only makes them nameable.

## Cost

Measured on the 12-core Mac mini at a load of 2–5, nothing else running
(2026-11-11):

| | Before (`core`) | After (`core`, `alloc`) |
|---|---|---|
| `bun scripts/std.ts`, sysroot made (once a toolchain) | 4.80 s | 5.49 s |
| `bun scripts/std.ts`, sysroot kept | 1.49 s | 1.50 s |
| The sysroot on disk | 43 MB | 52 MB (`liballoc.rmeta` 8.4 MB) |
| `@rust-js/std`'s files | 1,343,096 bytes | 1,343,096 bytes, the same |
| The corpus with std's iterator structs (`run forms 12`) | 37 s, its first run | 18 s; a program's build never reads this sysroot |

The package is byte for byte the same, and no table, net or corpus case
changes: the twin net and the corpus in both builds pass as before.

## Alternatives

- **Hand-write each provided method in the runtime**, as `$vecWriter`'s
  are (ADR 0392). Each would be a second copy of std's, which the twin net
  (ADR 1122) exists to hold to std's; #1156's decision (a) keeps
  the copies to the sources' own methods, where std's bodies reach memory.
- **Build `std` itself.** It needs a target's `sys` layer and a panic
  runtime, and nothing #1156 compiles is `std`'s own.
