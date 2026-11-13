//! A program's lines that show a library's writers that print as they
//! format: each print lands inside the line formatted so far, as Rust's
//! stdout's line writer has it, `x Outer { inner: [<step 1>1<step 2>, 2],`
//! (#1608); `{:#?}` too, which the program compiles the writers for itself
//! (ADR 0914). And a line the library's `print!` began, which the
//! program's `println!` ends (#1599). And a writer of the program's
//! that drops a library's value whose destructor prints, which the
//! program compiles itself: the print lands between its pieces.
use std::fmt;

use dep::{Dropping, Loud, Outer, Stepped, begin, many};

struct Show(u32);

impl fmt::Display for Show {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "a")?;
        drop(Loud(self.0));
        write!(f, "b")
    }
}

pub fn main() {
    println!("x {:?} y", Stepped(vec![1, 2]));
    println!("x {:#?} y", Stepped(vec![3]));
    println!("x {:?} y", Dropping(2));
    println!("x {:?} y", Outer);
    println!("x {:#?} y", Outer);
    println!("x {:?} y", vec![Stepped(vec![4]), Stepped(vec![5])]);
    println!("x {} y", Show(7));
    println!("x {:?} y", many());
    println!("x {:#?} y", many());
    begin("begun");
    println!("ended");
}
