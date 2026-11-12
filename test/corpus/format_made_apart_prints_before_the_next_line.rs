// A writer that prints, made into a `format!` in a body that may run while
// a line is formatted: every body that prints is one where a writer's calls
// can't be followed, `Stepped`'s `dyn Fn`, so `main`'s `format!` is made
// apart from the line (`$formatting`). What the writer prints is held in
// stdout's line as it's made, and written as the `format!` ends, before
// the line printed next, `<1>format L1`, as Rust's `LineWriter` writes it
// with that line (#1534). And one made by a method, `<2>then L2`.
use std::fmt;

struct Loud(u32);

impl fmt::Debug for Loud {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        print!("<{}>", self.0);
        write!(f, "L{}", self.0)
    }
}

struct Stepped(Vec<u32>);

impl fmt::Display for Stepped {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let next: Box<dyn Fn(u32) -> u32> = Box::new(|n| n + 1);
        write!(f, "{}", next(self.0[0]))
    }
}

fn main() {
    let s = format!("{:?}", Loud(1));
    println!("format {s}");
    println!("stepped {}", Stepped(vec![1]));
    let t = Loud(2).fmt_string();
    println!("then {t}");
}

trait FmtString {
    fn fmt_string(&self) -> String;
}

impl FmtString for Loud {
    fn fmt_string(&self) -> String {
        format!("{self:?}")
    }
}
