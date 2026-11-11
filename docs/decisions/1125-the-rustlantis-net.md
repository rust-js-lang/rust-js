# 1125. The Rustlantis net: custom MIR programs each night

Status: Accepted.

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a program Rustlantis
makes runs as native Rust does, or the JS is wrong.

## Context

[ADR 0092](0092-generated-programs.md)'s #1085 amendment took
Rustlantis's ideas into rust-js's own generator rather than the tool:
its programs are custom MIR, mostly raw pointers, `transmute` and unions,
whose failures would be those of programs no one writes. rust-js has
since grown to handle far more unsafe code, and #1349–#1352 sat in the
MIR space Rustlantis aims at, so it was measured (m3-max, 2026-11-11):
2,000 programs of its default size and of a small one, each natively for
wasm32 and as JS in both builds.

Its default programs, 12,000 lines, met a refusal 96% of the time, the
`bswap` intrinsic in 958 of 1,000 (#1384). Its small ones, three functions
of eight blocks, about 150 lines, ran: 395 of 1,000 as native Rust does,
281 refused, 299 threw and 25 printed otherwise. Of those, two wrong
answers of ordinary Rust no net had found: a wide literal cast through a
lossy `Number`, `u64::MAX as u32` printing 0 (#1380, every one of the 26
wrong answers), and a raw pointer to a by-value parameter, a cell its
callers never make (#1381, about 40 throws). Most of the rest, about 240
throws, were custom MIR's use of a local before any write of it whole,
which Rust can't write, refused now (#1382).

## Decision

**The nightly runs 1,000 of Rustlantis's small programs from the day's
seed, each natively for wasm32 and as JS in both builds**
(`test/rustlantis.test.ts`, `test/rustlantis.ts`).

- **The generator.** Rustlantis at a pinned commit with rust-js's patch,
  `test/rustlantis/wasm32.patch`: a `usize`, an `isize` and a pointer of 4
  bytes, as rust-js's target's, where the tool takes its host's, which
  made native wasm32 refuse its `transmute`s and its `usize` `match`es'
  arms meet. Built by the pinned toolchain with its bootstrap, no other
  toolchain, once a machine in `.cache/rustlantis/`, 512 MB at most, an
  earlier build removed as a new one starts. The config, small programs,
  is `test/rustlantis/config.toml`; each program dumps its locals with
  `{:?}` (`-d`).
- **The oracle.** Native `wasm32-wasip1`, rust-js's `usize`, run by
  `test/wasi-run.ts`, against the JS of each build run as the corpus's
  (`test/corpus-run.ts`): stdout, stderr and how it ended.
- **What it knows.** `test/rustlantis-known.txt`, by issue: refusals by
  their message, an open issue's or `custom-mir`, a form only custom MIR
  writes, refused for good; wrong answers by their program's source;
  throws by the JS's error; crashes by their message. Anything else fails
  the night with the command that makes it again. A fixed issue takes
  its lines off, so the list only shrinks. `custom-mir` is said of a
  refusal only once ordinary Rust is shown not to reach it: #1382's;
  "arithmetic checked for overflow, from its MIR", as rust-js compiles
  the crate with overflow checks off, a constant's and an inline
  constant's are rustc's to evaluate, and a std body's wraps (ADR 0899),
  so only `#[rustc_inherit_overflow_checks]`, behind a feature, reaches
  it; and "a `&mut` of a number kept or given, from its MIR", of seed
  341's `&mut (*r)` of a shared `&i16`, which the borrow checker refuses.
- **Where it runs.** The nightly, 4 parts of `scripts/parts.ts`, and
  `bun run nightly`; a push runs the list's own test. Of 1,000 seeds, 37 s
  in 12 parts on the 14-core Mac, the generator built once in 10 s; it
  counts its decisions as every nightly job does (ADR 1124).
- **Its default size** waits on #1384: until `bswap`, `fma` and
  `arith_offset` are supported, its programs are refusals.

## Consequences

- A wrong answer of ordinary Rust that only custom MIR's shapes reach,
  #1380's literal casts or #1381's parameter, is found the night it's
  made, of programs no hand or template wrote.
- Custom MIR reaches what Rust can't write; a failure is first asked
  whether ordinary Rust reaches it, as #1380 and #1381 were. One that
  doesn't is refused, as #1382's, never approximated.
- Rustlantis's generation is its own: a new commit, or a change to the
  patch or the config, gives the seeds other programs, and builds anew.
