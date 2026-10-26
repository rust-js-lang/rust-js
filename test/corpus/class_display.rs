// A class's `impl Display` is its prototype's `toString` (#358): `{p}` and
// `p.to_string()` are JS's text of it, a `T: Display` given it shows it by
// `String`, a `dyn Display` too, and another `Display`'s `self.at.fmt(f)`
// calls it. What Rust does with it is what it does natively.

use std::fmt;

#[cfg_attr(rust_js, rust_js::class)]
pub struct Position {
    row: i32,
    column: i32,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Position {
    pub fn new(row: i32, column: i32) -> Self {
        Position { row, column }
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.row < 0 {
            return write!(f, "off the board");
        }
        write!(f, "{},{}", self.row, self.column)
    }
}

struct Move {
    piece: char,
    at: Position,
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} to ", self.piece)?;
        self.at.fmt(f)
    }
}

fn listed<T: fmt::Display>(items: &[T]) -> String {
    items.iter().map(T::to_string).collect::<Vec<_>>().join("; ")
}

fn main() {
    let positions = [Position::new(0, 1), Position::new(-1, 3), Position::new(12, 7)];
    println!("{} [{:>6}] [{:<6}]", positions[0], positions[2], positions[0]);
    println!("{}", listed(&positions));
    let shown: Vec<&dyn fmt::Display> = vec![&positions[1], &3];
    for item in shown {
        println!("{item}");
    }
    let text = positions[2].to_string();
    println!(
        "{} {}",
        text.len(),
        Move {
            piece: 'Q',
            at: Position::new(4, 4)
        }
    );
}
