//! A library whose writers take a placeholder's options, as its own code
//! asks one, but which asks no `{:x?}`, so its JS writes an integer's
//! `{:?}` in decimal alone: its consumer that asks it compiles its own of
//! the writer that passes its `Formatter` on (#1026).
use std::fmt;

pub struct Wrap(pub u32);

impl fmt::Debug for Wrap {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

/// Padded by its own width, which makes its writers take options.
pub struct Name(pub &'static str);

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad(self.0)
    }
}
