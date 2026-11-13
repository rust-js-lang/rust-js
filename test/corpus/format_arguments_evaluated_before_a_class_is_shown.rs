// Every argument of a format evaluated before any is formatted, as Rust's
// `format_args!` takes them: a later argument's call before a class's
// `Display::fmt`, which a format before it runs. The later argument ran
// after it, `${loud(5)} ${q.toString()} ${loud(6)}` (#1676).
use std::fmt;

#[cfg_attr(rust_js, rust_js::class)]
pub struct Position {
    row: i32,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Position {
    pub fn new(row: i32) -> Self {
        Position { row }
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        println!("fmt {}", self.row);
        write!(f, "{}", self.row)
    }
}

fn loud(n: i32) -> i32 {
    println!("arg {n}");
    n
}

fn main() {
    let q = Position::new(4);
    let joined = format!("{} {} {}", loud(5), q, loud(6));
    println!("{joined}");
    println!("{} {} {} {}", q, loud(7), q, loud(8));
    let r = Position::new(9);
    println!("{:?} {r} {}", loud(10), loud(11));
}
