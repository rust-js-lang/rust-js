// A writer function given a `fn` pointer of a writer, its `Formatter`
// given on to it, `self.format(f, fmt::Display::fmt)`: a std formatting
// trait's `fmt` taken as a pointer, where a writer is shown into a writer,
// is the function that writes into the sink it claims, each piece by the
// `Formatter`'s options, a width's fill chars apart, and so is the crate's
// own; called plainly, each gives its text (#1652, ADR 1135).
use std::fmt::{self, Write};

struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

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

fn main() {
    let j = Joined(&[1, 20, 300], ", ");
    println!("{j} {j:>5} {j:x} {j:#06x} {j:?}");
    let mut log = Log(Vec::new());
    write!(log, "{j}|{j:x}|{j:?}|{j:>4}").unwrap();
    println!("{:?}", log.0);
}
