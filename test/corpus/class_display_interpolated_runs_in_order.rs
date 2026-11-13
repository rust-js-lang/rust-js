//@ library-differs
// A class's `Display::fmt`, its prototype's `toString`, run where a
// template interpolates its value: before a later format argument, as
// Rust's `println!` and `format!` write each in turn, and where its text is
// discarded. The interpolation was taken for a plain read, so a later
// argument's call ran first, and a minifier, which takes a `toString` to run
// nothing, moved a held argument past it (#1664).
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
    let p = Position::new(1);
    println!("{} {}", p, loud(2));
    let text = format!("{p}/{}", loud(3));
    println!("{text}");
    // Formatted for its `Display`'s print alone, its text discarded.
    let _ = format!("{}", p);
    let _ = p.to_string();
    println!("{:>4}|", p);
}
