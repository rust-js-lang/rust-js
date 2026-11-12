// A writer that prints as it formats (#1490) and returns `Err`, given to
// `write!` into a `String`: the `write!` gives the `Err` back, the text
// written before it kept in the `String`, as it does of one that doesn't
// print (ADR 0187); and its print comes where it ran, before the line
// printed next (#1520). Inside a line, into a `String` a writer makes,
// its print lands inside that line.
use std::fmt::{self, Write};

struct Refused;

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("half")?;
        print!("(refusing)");
        Err(fmt::Error)
    }
}

struct Fine(u8);

impl fmt::Display for Fine {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        print!("(fine)");
        write!(f, "whole {}", self.0)
    }
}

/// Writes one of each into a `String` of its own as it's shown.
struct Both;

impl fmt::Display for Both {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut buf = String::new();
        let refused = write!(buf, "<{}>", Refused).is_err();
        let fine = write!(buf, "<{}>", Fine(2)).is_ok();
        write!(f, "both {refused} {fine} [{buf}]")
    }
}

fn main() {
    let mut buf = String::new();
    let error = write!(buf, "{}", Refused).expect_err("a refused write fails");
    println!("{error:?} {error} [{buf}]");
    let r = write!(buf, " and {}", Fine(1));
    println!("{r:?} [{buf}]");
    println!("line {} end", Both);
}
