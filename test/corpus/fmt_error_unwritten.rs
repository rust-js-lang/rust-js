// A writer that fails having written nothing, its `Formatter` unread:
// `write!` to a `String` gives back its `Err`, the string holding what was
// written before the failing piece, and `to_string` panics as std's does.
use std::fmt::{self, Write};

struct Bad;

impl fmt::Display for Bad {
    fn fmt(&self, _: &mut fmt::Formatter) -> fmt::Result {
        Err(fmt::Error)
    }
}

fn main() {
    let mut s = String::new();
    let written = write!(s, "a{}b", Bad);
    println!("{:?} {:?}", written, s);
    let mut t = String::from("<");
    let again = write!(t, "{Bad}");
    println!("{:?} {:?}", again, t);
}
