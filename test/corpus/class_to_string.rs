// A class's `impl Display` is its prototype's `toString` (#358), in a
// crate that gives no `Formatter` options: `{p}` and `p.to_string()` are
// JS's text of it, `${p}`, and a `T: Display` shows it by `String`. What
// Rust does with it is what it does natively.

use std::fmt;

#[cfg_attr(rust_js, rust_js::class)]
pub struct QueenPosition {
    row_index: i32,
    column_index: i32,
}

#[cfg_attr(rust_js, rust_js::class)]
impl QueenPosition {
    pub fn new(row_index: i32, column_index: i32) -> Self {
        QueenPosition {
            row_index,
            column_index,
        }
    }

    pub fn left_diagonal(&self) -> i32 {
        self.row_index - self.column_index
    }
}

impl fmt::Display for QueenPosition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{},{}", self.row_index, self.column_index)
    }
}

fn listed<T: fmt::Display>(items: &[T]) -> String {
    items.iter().map(T::to_string).collect::<Vec<_>>().join(" ")
}

fn main() {
    let solution = [
        QueenPosition::new(0, 1),
        QueenPosition::new(1, 3),
        QueenPosition::new(2, 0),
    ];
    println!("{} {}", solution[0], solution[1].left_diagonal());
    println!("{}", listed(&solution));
    let shown: &dyn fmt::Display = &solution[2];
    let text = solution[2].to_string();
    println!("{shown} {} {}", text == "2,0", text.len());
}
