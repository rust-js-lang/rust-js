// Rust's stderr is unbuffered: an `eprintln!` writes each piece as it's
// formatted, so what a writer prints to stdout meanwhile comes after the
// text written before it, `err line inner f` (#1490); and stdout's
// `LineWriter` holds a line's pieces till its newline, a partial one across
// a caught panic, and what's left at the end is written then.
// test/stream-order.test.ts holds the two streams' order against native.
use std::fmt;
struct Loud(&'static str);
impl fmt::Display for Loud {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        println!("inner {}", self.0);
        write!(f, "<{}>", self.0)
    }
}
struct Guard(&'static str);
impl Drop for Guard {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
struct Dropping;
impl fmt::Display for Dropping {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let _g = Guard("in fmt");
        write!(f, "dropping")
    }
}
struct Err2(&'static str);
impl fmt::Display for Err2 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        eprintln!("err {}", self.0);
        write!(f, "[{}]", self.0)
    }
}
struct Partial;
impl fmt::Display for Partial {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "half")?;
        panic!("mid-line");
    }
}
fn main() {
    println!("value: {} and {}", Loud("a"), Loud("b"));
    print!("start ");
    println!("{}", Loud("c"));
    println!("x {} y", Dropping);
    println!("out {}", Err2("e"));
    eprintln!("err line {}", Loud("f"));
    let caught = std::panic::catch_unwind(|| {
        println!("before {} after", Partial);
    });
    println!("caught {}", caught.is_err());
    print!("no newline at the end");
}
