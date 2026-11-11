//! A library that formats a `dyn Display`, which no consumer can compile
//! for its own, and writes nothing that fails: a format through one may
//! fail, whatever the library's own do (#1392).
use std::fmt::{self, Display, Write};

pub fn shown(t: &dyn Display) -> String {
    let mut out = String::new();
    match write!(out, "[{t}]") {
        Ok(()) => out,
        Err(fmt::Error) => format!("refused after {out:?}"),
    }
}
