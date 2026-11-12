// A stdout line a writer's print ends, as `println!` formats, is written by
// Rust's `LineWriter` there and then, so an `eprintln!` the writer makes
// after it comes after that line (#1490): of a writer inside another, of
// one that panics or fails after it, the panic caught, and of a `{:#?}`
// builder's field, through its adapter. test/stream-order.test.ts holds
// the two streams' order against native.
use std::fmt;

struct Noisy(u32);

impl fmt::Display for Noisy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<")?;
        println!("in {}", self.0);
        eprintln!("err {}", self.0);
        write!(f, "{}>", self.0)
    }
}

impl fmt::Debug for Noisy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "N")?;
        println!("dbg {}", self.0);
        eprintln!("derr {}", self.0);
        write!(f, "{}", self.0)
    }
}

struct Outer(u32);

impl fmt::Display for Outer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "o[")?;
        write!(f, "{}", Noisy(self.0))?;
        eprintln!("after {}", self.0);
        write!(f, "]")
    }
}

struct Panics;

impl fmt::Display for Panics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<")?;
        println!("p");
        eprintln!("err p");
        panic!("stopped");
    }
}

struct Fails;

impl fmt::Display for Fails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<")?;
        println!("q");
        eprintln!("err q");
        Err(fmt::Error)
    }
}

struct Lines(u32);

impl fmt::Debug for Lines {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "one\ntwo ")?;
        println!("lines {}", self.0);
        eprintln!("lerr {}", self.0);
        write!(f, "three")
    }
}

struct Spread(Lines);

impl fmt::Debug for Spread {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Spread").field("l", &self.0).finish()
    }
}

struct Pair(Noisy, Noisy);

impl fmt::Debug for Pair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Pair").field("a", &self.0).field("b", &self.1).finish()
    }
}

fn main() {
    println!("a {} b {} c", Noisy(1), Noisy(2));
    println!("nested {} end", Outer(3));
    let caught = std::panic::catch_unwind(|| println!("before {} after", Panics));
    println!(" then {}", caught.is_err());
    let failed = std::panic::catch_unwind(|| println!("before {} after", Fails));
    println!(" then {}", failed.is_err());
    println!("{:?}", Pair(Noisy(4), Noisy(5)));
    println!("{:#?}", Pair(Noisy(6), Noisy(7)));
    println!("{:#?}", Spread(Lines(8)));
    println!("done");
}
