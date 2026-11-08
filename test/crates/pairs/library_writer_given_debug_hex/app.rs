//! In hex where it's asked, and the name padded as the library pads it.
use dep::{Name, Wrap};

pub fn main() {
    println!("{:x?} {:X?} {:?}", Wrap(255), Wrap(255), Wrap(255));
    println!("[{:>6}]", Name("ann"));
}
