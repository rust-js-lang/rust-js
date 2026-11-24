//! Where values cross from JS into Rust, and what Rust checks of them
//! there (#2403, ADR 1192): every value a JS caller gives is checked against
//! its Rust type, so Rust never sees an impossible value, a `u32` of `-1`
//! or a variant no `match` arm has. Shallow first (`checks`): a function's
//! parameters of numbers, `bool`s, `char`s, strings, `()` and fieldless
//! enums, and `Option`s of them.

pub(crate) mod brands;
mod callbacks;
mod checks;
mod deep;
mod patterns;
mod results;
pub mod split;
mod type_tests;
