//! A library's generic functions that ask what its consumer's own code
//! doesn't: a `Chars`' rest and `{:#?}`. The consumer compiles
//! its own of each, given a value with a destructor (ADR 0914), from the
//! library's MIR, which has no THIR to say what it asks: what its MIR
//! calls says a `Chars`' rest is asked, and its library's manifest that
//! its writers take a `Formatter`'s options (#1325).
use std::fmt::Debug;

pub fn rest_after_one<T>(held: T, s: &str) -> String {
    let mut chars = s.chars();
    chars.next();
    drop(held);
    chars.as_str().to_string()
}

pub fn pretty<T: Debug>(held: T) -> String {
    format!("{:#?}", held)
}
