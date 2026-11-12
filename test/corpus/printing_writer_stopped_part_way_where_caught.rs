// A writer that prints as it formats (#1490) and then stops part-way, by
// a panic a `catch_unwind` catches or by an error: Rust's stdout keeps what
// the line had written, the writer's prints and text before it stopped
// too, and writes it before what's printed after. A writer of one inside
// its own text, which a print of the outer one follows; and one that
// catches the inner one's panic itself and writes on, after what the inner
// one wrote (#1521). One that never stops part-way, shown in a `Vec`.
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u32);

impl fmt::Debug for D {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        print!("!");
        write!(f, "D(")?;
        print!("?");
        if self.0 == 3 {
            panic!("three");
        }
        write!(f, "{})", self.0)
    }
}

struct Outer(D);

impl fmt::Display for Outer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "outer ")?;
        print!("<");
        write!(f, "{:?}", self.0)?;
        print!(">");
        write!(f, " end")
    }
}

/// Catches its part's panic and writes on.
struct Guarded(D);

impl fmt::Display for Guarded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "g[")?;
        let r = catch_unwind(AssertUnwindSafe(|| {
            let _ = write!(f, "{:?}", self.0);
        }));
        print!("<{}>", r.is_err());
        write!(f, "]")
    }
}

/// Prints, and never stops part-way.
struct Quiet;

impl fmt::Debug for Quiet {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        print!("q");
        Ok(())
    }
}

fn main() {
    println!("w {:?} y", D(2));
    let r = catch_unwind(|| println!("w {:?} y", D(3)));
    println!("{}", r.is_err());
    let r = catch_unwind(|| println!("x {} y", Outer(D(3))));
    println!("{}", r.is_err());
    let r = catch_unwind(|| println!("x {} y", Outer(D(4))));
    println!("{}", r.is_err());
    let r = catch_unwind(|| {
        print!("before ");
        println!("{:?}", D(3));
    });
    println!("{}", r.is_err());
    println!("x {} y", Guarded(D(3)));
    println!("x {} y", Guarded(D(4)));
    println!("{:?}", vec![Quiet, Quiet]);
}
