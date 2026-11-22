//! The compiler's owned output (ADR 1181): what rust-js writes, as data of
//! its own, with no rustc in it. The front end (`src/lower/`) builds it and
//! the printer reads it; this crate knows neither.

pub mod collections;
pub mod decisions;
pub mod lines;
