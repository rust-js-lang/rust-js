// What the crate's functions that never unwind are, read from their MIR
// (ADR 0902): one that only prints numbers and text is followed by the drop
// of what's held, no `try` round it; one that may panic, by an index past
// the end or a `Display` of the crate's that panics, still drops what's
// held as it unwinds.
use std::fmt;
use std::panic;

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Bad;

impl fmt::Display for Bad {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        panic!("bad display")
    }
}

fn greet(name: &str, n: u32) {
    println!("hello {name} {n}");
}

fn pick(items: &[u32], at: usize) -> u32 {
    items[at]
}

fn show(bad: &Bad) {
    println!("{bad}");
}

fn main() {
    let _a = Loud("a");
    greet("ann", 1);
    let r = panic::catch_unwind(|| {
        let _b = Loud("b");
        greet("bob", 2);
        pick(&[1, 2], 5)
    });
    println!("picked {}", r.is_err());
    let r = panic::catch_unwind(|| {
        let _c = Loud("c");
        show(&Bad);
    });
    println!("shown {}", r.is_err());
    greet("cat", 3);
}
