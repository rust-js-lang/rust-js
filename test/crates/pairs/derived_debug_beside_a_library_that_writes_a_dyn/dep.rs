//! A library that writes a `dyn` it's given into a writer a piece at a
//! time, as serde_json's error, and so the Canvas app's core library, does:
//! its contract says it writes a `dyn` it's given (#1772).
use std::fmt::{self, Write};

/// A writer of its own, which keeps each piece written.
pub struct Log(pub Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

pub fn describe(into: &mut Log, what: &dyn fmt::Debug) -> fmt::Result {
    write!(into, "[{what:?}]")
}

pub fn label() -> &'static str {
    "offline"
}
