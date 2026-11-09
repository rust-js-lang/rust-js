//! A writer whose `write_str` fails once it's full.
use std::fmt;

pub struct Full(pub usize);

impl fmt::Write for Full {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if s.len() > self.0 {
            return Err(fmt::Error);
        }
        self.0 -= s.len();
        Ok(())
    }
}
