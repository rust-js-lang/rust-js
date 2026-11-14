//! A library that writes a `dyn Debug` it's given into a writer of its own
//! a piece at a time (#1774).
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
