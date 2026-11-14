// A writer of the crate's own that gives its `Formatter` to the `fmt` of an
// impl of a reference to, or a `Box` of, its own type, `Display::fmt(&self.0,
// f)` and `LowerHex::fmt`: each that impl's, written into a writer a piece
// at a time, and formatted plainly (#1798).
use std::fmt::{self, Write};

struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

struct ByRef(u8);

impl fmt::LowerHex for &ByRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "h{:x}", self.0)
    }
}

impl fmt::Display for &ByRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "r{}", self.0)
    }
}

struct Boxed(u8);

impl fmt::Display for Box<Boxed> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "b{}", self.0)
    }
}

struct Outer<'a>(&'a ByRef, Box<Boxed>);

impl fmt::Display for Outer<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<")?;
        fmt::Display::fmt(&self.0, f)?;
        f.write_str("|")?;
        fmt::Display::fmt(&self.1, f)?;
        f.write_str("|")?;
        fmt::LowerHex::fmt(&self.0, f)?;
        f.write_str(">")
    }
}

// Formatted plainly alone, never written into a writer, so its writer
// isn't framed.
struct Plain<'a>(&'a ByRef, Box<Boxed>);

impl fmt::Display for Plain<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(&self.0, f)?;
        fmt::Display::fmt(&self.0, f)?;
        fmt::Display::fmt(&self.1, f)
    }
}

fn main() {
    let r = ByRef(26);
    let o = Outer(&r, Box::new(Boxed(2)));
    println!("{o} {:x} {:#x}", &r, &r);
    println!(
        "{} {}",
        Plain(&r, Box::new(Boxed(5))),
        Plain(&r, Box::new(Boxed(6))).to_string()
    );
    let mut log = Log(Vec::new());
    write!(log, "{o}{:x}", &r).unwrap();
    println!("{:?}", log.0);
    println!("{}", o.to_string());
}
