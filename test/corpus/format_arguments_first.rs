// Rust evaluates each argument of a format string before it formats any:
// a `Debug` that prints, of an argument before one made by a call that
// prints too, prints after it, whatever order the string reads them in
// (ADR 0377).

use std::fmt;

struct Loud(&'static str);

impl fmt::Debug for Loud {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        println!("fmt {}", self.0);
        write!(f, "Loud({})", self.0)
    }
}

fn make(n: u8) -> (u8, u8) {
    println!("make {n}");
    (n, n + 1)
}

fn named(s: &'static str) -> Loud {
    println!("named {s}");
    Loud(s)
}

fn main() {
    let a = Loud("a");
    println!("{:?} {:?}", a, make(1));
    println!("{:?} {:?} {}", named("b"), make(2), make(3).0);
    println!("{1:?} {0}", named("c").0, make(4));
    println!("{1} {0}", named("d").0, make(5).0);
    println!("{}", make(6).1);
}
