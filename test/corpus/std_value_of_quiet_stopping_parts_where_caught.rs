//@ library-differs
// A `Vec`'s `Debug` in a line, of parts whose writers print nothing but may
// panic, the panic caught: by an assertion, by stepping a builder's
// iterator, through a writer of the crate's, through a `dyn Debug` read
// from a field, through a derived `Debug`'s field, and through a helper
// that formats what panics. Rust's stdout keeps what std's builder wrote
// before the part that stops, `x [Q1, Q2, `, as each piece is written as
// it's taken (#1561). These were refused (#1530).
use std::fmt::{self, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};

struct Quiet(u8);

impl fmt::Debug for Quiet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        assert!(self.0 < 3, "too big");
        write!(f, "Q{}", self.0)
    }
}

struct Steps(u32);

impl Iterator for Steps {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        assert!(self.0 < 2, "stepped too far");
        self.0 += 1;
        Some(self.0)
    }
}

struct Listed(u32);

impl fmt::Debug for Listed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(Steps(self.0)).finish()
    }
}

struct Short;

impl fmt::Write for Short {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        assert!(s.len() < 3, "too long");
        Ok(())
    }
}

struct Written(u32);

impl fmt::Debug for Written {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _ = write!(Short, "{}", self.0);
        write!(f, "w{}", self.0)
    }
}

struct Boxed(Box<dyn fmt::Debug>);

impl fmt::Debug for Boxed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

#[derive(Debug)]
struct Holds {
    quiet: Quiet,
}

fn label(n: u8) -> String {
    format!("{:?}", Quiet(n))
}

struct Labelled(u8);

impl fmt::Debug for Labelled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", label(self.0))
    }
}

fn main() {
    let r = catch_unwind(|| println!("x {:?} y", vec![Quiet(1), Quiet(2), Quiet(3)]));
    println!(" caught {}", r.is_err());
    let r = catch_unwind(|| println!("x {:?} y", vec![Listed(0), Listed(1)]));
    println!(" caught {}", r.is_err());
    let r = catch_unwind(|| println!("x {:?} y", vec![Written(1), Written(100)]));
    println!(" caught {}", r.is_err());
    let boxed = vec![Boxed(Box::new(Quiet(1))), Boxed(Box::new(Quiet(4)))];
    let r = catch_unwind(AssertUnwindSafe(|| println!("x {:?} y", boxed)));
    println!(" caught {}", r.is_err());
    let r = catch_unwind(|| println!("x {:#?} y", vec![Holds { quiet: Quiet(1) }, Holds { quiet: Quiet(5) }]));
    println!(" caught {}", r.is_err());
    let r = catch_unwind(|| println!("x {:?} y", vec![Labelled(1), Labelled(6)]));
    println!(" caught {}", r.is_err());
    println!("x {:?} y", vec![Quiet(1), Quiet(2)]);
}
