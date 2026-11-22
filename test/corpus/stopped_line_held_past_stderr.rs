// A line a caught panic stopped as it was formatted: what it wrote before
// the panic stays in std's stdout `LineWriter`, held past the write to
// standard error after it, till the next line's newline (#2347, #2319).
use std::fmt;
use std::panic;

struct Breaks;

impl fmt::Display for Breaks {
    fn fmt(&self, _: &mut fmt::Formatter) -> fmt::Result {
        panic!("stopped");
    }
}

fn main() {
    let _ = panic::catch_unwind(|| println!("before {} after", Breaks));
    eprintln!("err");
    println!("end");
}
