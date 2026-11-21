//! The library's error, holding a `dyn Error` it or this crate made,
//! written by its `Debug` and `Display` into a writer of this crate's own:
//! each piece a call of the writer's, as natively (#2250).
use std::error::Error;
use std::fmt::{self, Write};

use dep::Wrapped;

struct Pieces(Vec<String>);

impl Write for Pieces {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

#[derive(Debug)]
struct Mine {
    why: &'static str,
}

impl fmt::Display for Mine {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "mine: {}", self.why)
    }
}

impl Error for Mine {}

/// An error only std's `io::Error::other` makes a `dyn` of, by the
/// conversion it's given.
#[derive(Debug)]
struct Inner(u8);

impl fmt::Display for Inner {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "inner {}", self.0)
    }
}

impl Error for Inner {}

fn show(name: &str, wrapped: Wrapped) {
    let mut debug = Pieces(Vec::new());
    write!(debug, "{wrapped:?}").unwrap();
    let mut display = Pieces(Vec::new());
    write!(display, "{wrapped}").unwrap();
    println!("{name}: {:?} {:?}", debug.0, display.0);
}

pub fn main() {
    show("message", Wrapped::new("a \"quoted\" message"));
    show("string", Wrapped::new(String::from("owned\n")));
    show("library's", dep::coded(7));
    show("library's parse", dep::parsed("x"));
    show("mine", Wrapped::new(Mine { why: "it's \"mine\"" }));
    show("io", Wrapped::new(std::io::Error::other("other")));
    show("io of mine", Wrapped::new(std::io::Error::other(Inner(9))));
}
