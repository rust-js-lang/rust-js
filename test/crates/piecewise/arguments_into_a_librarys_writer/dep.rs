//! A library's functions that write an `fmt::Arguments` they're given
//! into a writer, itself and through another, which its contract says
//! (`written_arguments`); and a writer of its own whose own `write_fmt`
//! passes what it's given on to the writer it holds, which its dictionary's
//! `writeFmt` is (#1652, step 6 of ADR 1135).
use std::fmt::{self, Write};

pub fn log_into<W: Write>(w: &mut W, args: fmt::Arguments<'_>) -> fmt::Result {
    w.write_fmt(args)
}

pub fn relay<W: Write>(w: &mut W, args: fmt::Arguments<'_>) -> fmt::Result {
    w.write_str(">")?;
    log_into(w, args)
}

/// Counts its `write_fmt` calls, and passes each on to the writer it holds.
pub struct Tee<W>(pub W, pub u32);

impl<W: Write> Write for Tee<W> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.write_str(s)
    }

    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> fmt::Result {
        self.1 += 1;
        self.0.write_fmt(args)
    }
}

/// Counts its `write_fmt` calls, and writes what it's given into itself by
/// std's `fmt::write`, a piece at a time.
pub struct Counted(pub Vec<String>, pub u32);

impl Write for Counted {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }

    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> fmt::Result {
        self.1 += 1;
        fmt::write(self, args)
    }
}
