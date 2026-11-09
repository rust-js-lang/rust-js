//! A library's formatting impls that forward by path, `fmt::LowerHex::fmt(
//! &self.0, f)`, `fmt::Display::fmt(self, f)`, the `Formatter` given on,
//! its options with it: the corpus case `fmt_trait_fmt_called_by_path.rs`'s
//! items as a library's, which the metamorphic net's `library` rewrite found
//! losing them (#1164).
use std::fmt::{self, Write};

pub struct Hexy(pub u32);

impl fmt::LowerHex for Hexy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.write_str("hex:")?;
        }
        fmt::LowerHex::fmt(&self.0, f)
    }
}

impl fmt::Debug for Hexy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(self, f)
    }
}

pub struct Shown(pub u32);

impl fmt::Display for Shown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl fmt::Debug for Shown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

// libm's shape: a generic wrapper whose `Debug` forwards to its `LowerHex`.
pub struct Hexf<T>(pub T);

impl fmt::LowerHex for Hexf<u8> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:02x}", self.0)
    }
}

impl fmt::LowerHex for Hexf<(u8, u8)> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:x}, {:x})", Hexf(self.0.0), Hexf(self.0.1))
    }
}

impl<T> fmt::Debug for Hexf<T>
where
    Hexf<T>: fmt::LowerHex,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(self, f)
    }
}

// A writer that fails past its room: the callee's error is the caller's.
pub struct Full(pub usize);

impl Write for Full {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if s.len() > self.0 {
            return Err(fmt::Error);
        }
        self.0 -= s.len();
        Ok(())
    }
}

