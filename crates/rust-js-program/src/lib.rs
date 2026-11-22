//! The compiler's owned output (ADR 1181): what rust-js writes, as data of
//! its own, with no rustc in it. The front end (`src/lower/`) builds it and
//! the printer reads it; this crate knows neither.

pub mod casing;
pub mod collections;
pub mod comments;
pub mod css_modules;
pub mod decisions;
pub mod js;
pub mod library;
pub mod lines;
pub mod link;
pub mod manifest;
pub mod names;
pub mod packages;
pub mod paths;
pub mod prepare;
pub mod program;
pub mod reachability;
pub mod runtime;
pub mod settings;
