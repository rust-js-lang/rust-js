// Rust evaluates each argument of a format string before it formats any:
// a `Debug` that prints, of an argument before one made by a call that
// prints too, prints after it, whatever order the string reads them in
// (ADR 0377). Where a line is written a piece at a time, through stdout's
// line as a `Debug` that prints is, or into a text whose formatting may
// fail, an argument made by a call that prints is made before the text
// written ahead of it, and before a formatting that fails stops the rest.

use std::fmt::{self, Write};

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

/// Its formatting fails.
struct Refused;

impl fmt::Display for Refused {
    fn fmt(&self, _: &mut fmt::Formatter) -> fmt::Result {
        Err(fmt::Error)
    }
}

/// Its `Debug` calls `shout`, which so prints through the line.
struct Echo;

impl fmt::Debug for Echo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        shout(0);
        write!(f, "Echo")
    }
}

fn shout(n: u32) -> u32 {
    println!("shout {n}");
    n
}

fn noted(n: u32) -> u32 {
    println!("noted {n}");
    n
}

fn main() {
    let a = Loud("a");
    println!("{:?} {:?}", a, make(1));
    println!("{:?} {:?} {}", named("b"), make(2), make(3).0);
    println!("{1:?} {0}", named("c").0, make(4));
    println!("{1} {0}", named("d").0, make(5).0);
    println!("{}", make(6).1);
    println!("lead {} {:?}", shout(8), Echo);
    let mut text = String::new();
    let written = write!(text, "<{}|{}>", Refused, noted(7));
    println!("{} {text:?}", written.is_err());
}
