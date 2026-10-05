//@ run-fail: a formatting trait implementation returned an error when the underlying stream did not
// `println!` of a value whose `Display` fails panics, as std's does, and
// what it wrote before is printed.
use std::fmt::{self, Display};

struct Hours(u32);

impl Display for Hours {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0 > 23 {
            return Err(fmt::Error);
        }
        write!(f, "{:02}h", self.0)
    }
}

struct Span(Hours, Hours);

impl Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)?;
        f.write_str("-")?;
        write!(f, "{}", self.1)
    }
}

fn main() {
    println!("before");
    println!("<{}>", Span(Hours(1), Hours(99)));
    println!("after");
}
