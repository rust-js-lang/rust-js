//! A library's generic function its consumer compiles, given a value to
//! drop that it takes nothing for, which calls a private helper that may
//! fail: the helper, compiled here too, is listed in no manifest, so only
//! the consumer's reading of its MIR finds it failing (ADR 0914, amended;
//! #1043).
use std::fmt::{self, Write};

pub struct Hour(pub u32);

impl fmt::Display for Hour {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0 > 23 {
            return Err(fmt::Error);
        }
        write!(f, "{:02}h", self.0)
    }
}

fn write_hour(out: &mut String, hour: &Hour) -> fmt::Result {
    write!(out, "{hour}")
}

/// The helper's `Err` tested as a value, which a call taken to never fail
/// throws past.
pub fn stamp<T>(value: T, hour: &Hour) -> String {
    let mut out = String::new();
    let shown = match write_hour(&mut out, hour) {
        Ok(()) => out,
        Err(_) => String::from("--"),
    };
    drop(value);
    shown
}
