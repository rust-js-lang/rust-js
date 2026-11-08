//! A library whose own code formats nothing with a placeholder's options,
//! so its writers' JS takes none: its consumer that gives them compiles
//! its own of each writer that reads them, and imports the rest (#1026).
use std::fmt;

/// Its items, each written by the callback it's given its `Formatter`.
pub struct Joined<'a>(pub &'a [u32], pub &'a str);

impl Joined<'_> {
    pub fn format(&self, f: &mut fmt::Formatter, cb: fn(&u32, &mut fmt::Formatter) -> fmt::Result) -> fmt::Result {
        for (i, x) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str(self.1)?;
            }
            cb(x, f)?;
        }
        Ok(())
    }
}

/// A writer that only writes its text, which a placeholder's options
/// leave alone: the library's own.
pub fn quoted(x: &u32, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "'{x}'")
}

impl fmt::Display for Joined<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.format(f, fmt::Display::fmt)
    }
}

impl fmt::LowerHex for Joined<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.format(f, fmt::LowerHex::fmt)
    }
}

impl fmt::Debug for Joined<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.format(f, quoted)
    }
}

/// A builder held in a variable, stepped in a loop.
pub struct Row(pub Vec<u32>);

impl fmt::Debug for Row {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut row = f.debug_tuple("Row");
        for x in &self.0 {
            row.field(x);
        }
        row.finish()
    }
}

/// One that writes its text alone: no width pads it, in Rust too.
pub struct Hour(pub u32);

impl fmt::Display for Hour {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:02}h", self.0)
    }
}

/// A writer that writes through a `Formatter` a struct holds, its
/// `fmt::Write`'s `write_str`, compiled here with it.
pub struct Quoted<'a, 'b>(pub &'a mut fmt::Formatter<'b>);

impl fmt::Write for Quoted<'_, '_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        fmt::Display::fmt(&s.escape_debug(), self.0)
    }
}

pub struct Message(pub String);

impl fmt::Debug for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use fmt::Write;
        f.write_char('"')?;
        Quoted(f).write_fmt(format_args!("{}", self.0))?;
        f.write_char('"')
    }
}
