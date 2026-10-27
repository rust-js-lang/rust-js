// A writer that may fail on either of two branches having written nothing:
// its string, never written, is a `const`, made before the `try` whose
// `catch` gives it as what was written (ADR 0187, amended), and `write!`
// gives back the `Err`, the string holding what came before the failing
// piece.
use std::fmt::{self, Write};

struct Strict(bool);

impl fmt::Display for Strict {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0 {
            return Err(fmt::Error);
        }
        if f.alternate() {
            return Err(fmt::Error);
        }
        Ok(())
    }
}

struct Wrapped(Strict, u32);

impl fmt::Display for Wrapped {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.1 > 2 {
            write!(f, "{}", self.0)?;
        }
        Ok(())
    }
}

fn main() {
    for strict in [true, false] {
        let mut s = String::new();
        let written = write!(s, "x{}y{:#}z", Strict(strict), Strict(false));
        println!("{written:?} {s:?}");
        let mut t = String::from("<");
        let wrapped = write!(t, "{}>{}", Wrapped(Strict(strict), 3), Wrapped(Strict(true), 1));
        println!("{wrapped:?} {t:?}");
    }
}
