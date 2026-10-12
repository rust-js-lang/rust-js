// A writer's `write!` of what may fail, piece by piece: what it wrote
// before the failing `{}` stays written, as Rust's does.
use std::fmt::{self, Display, Write};

struct Hours(u32);

impl Display for Hours {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0 > 23 {
            return Err(fmt::Error);
        }
        write!(f, "{}h", self.0)
    }
}

struct Pair(Hours, Hours);

impl Display for Pair {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<{} to {}>", self.0, self.1)
    }
}

fn main() {
    let mut t = String::new();
    let r = write!(t, "{}", Pair(Hours(9), Hours(30)));
    let mut u = String::new();
    let s = write!(u, "{}", Pair(Hours(9), Hours(10)));
    println!("{} {t:?} {} {u:?}", r.is_err(), s.is_err());
}
