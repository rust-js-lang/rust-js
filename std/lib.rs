//! `@rust-js/std`: std's own bodies, each lowered from the MIR the toolchain
//! ships, into the module of this crate named as std's is (the std-bodies
//! decision). The crate has nothing of its own: `bun run std` compiles it
//! with `--std-bodies`, which gives it std's bodies, and writes each
//! module's JS beside this file, with the manifest the compiler reads.
#![no_std]

/// `core::iter`'s: the adapters and `Iterator`'s provided methods.
pub mod iter {}

/// `core::ops`'s: what a compiled body of `iter`'s calls of it, `try_fold`'s
/// `NeverShortCircuit` (ADR 0913).
pub mod ops {}
