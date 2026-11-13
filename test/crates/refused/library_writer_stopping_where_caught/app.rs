//! A program that catches a panic, past which it prints on, and shows a
//! library's writer that may stop part-way in stdout's line: Rust's stdout
//! keeps what it wrote before it stopped, `x [!L1, !L2, !`, where the
//! library's JS, which catches nothing, makes its text whole, so the
//! program is refused, naming #1530 and what to write (#1608): where a
//! line shows it, a writer of the program's in a line shows it, and a line
//! shows a value only a caller knows the type of.
use std::fmt;
use std::panic::catch_unwind;

use dep::Loud;

struct Wrapped(Loud);

impl fmt::Display for Wrapped {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "w{:?}", self.0)
    }
}

fn show<T: fmt::Debug>(value: &T) {
    println!("shown {value:?}");
}

pub fn main() {
    let caught = catch_unwind(|| println!("x {:?} y", vec![Loud(1), Loud(2), Loud(3)]));
    println!(" caught {}", caught.is_err());
    let caught = catch_unwind(|| println!("x {} y", Wrapped(Loud(3))));
    println!(" caught {}", caught.is_err());
    let caught = catch_unwind(|| show(&Loud(3)));
    println!(" caught {}", caught.is_err());
}
