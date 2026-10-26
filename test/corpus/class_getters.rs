// A class's method marked `#[rust_js::getter]` is its prototype's getter
// (#357): `Object.defineProperty(Queen.prototype, "leftDiagonal", { get()
// { .. } })`, read `queen.leftDiagonal`, in the order Rust calls it, its
// side effects too; as a value, the arrow of its read. What Rust does with
// it is what it does natively.

use std::cell::Cell;

#[cfg_attr(rust_js, rust_js::class)]
pub struct Queen {
    row: i32,
    column: i32,
    reads: Cell<u32>,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Queen {
    pub fn new(row: i32, column: i32) -> Self {
        Queen {
            row,
            column,
            reads: Cell::new(0),
        }
    }

    /// The same on each position of its left (\) diagonal.
    #[cfg_attr(rust_js, rust_js::getter)]
    pub fn left_diagonal(&self) -> i32 {
        self.reads.set(self.reads.get() + 1);
        self.row - self.column
    }

    /// The same on each position of its right (/) diagonal.
    #[cfg_attr(rust_js, rust_js::getter)]
    pub fn right_diagonal(&self) -> i32 {
        self.reads.set(self.reads.get() + 1);
        self.row + self.column
    }

    #[cfg_attr(rust_js, rust_js::getter)]
    pub fn label(&self) -> String {
        format!("{},{}", self.row, self.column)
    }

    pub fn threatens(&self, other: &Queen) -> bool {
        self.row == other.row
            || self.column == other.column
            || self.left_diagonal() == other.left_diagonal()
            || self.right_diagonal() == other.right_diagonal()
    }

    pub fn read_count(&self) -> u32 {
        self.reads.get()
    }
}

fn main() {
    let queens = [
        Queen::new(0, 1),
        Queen::new(1, 3),
        Queen::new(2, 0),
        Queen::new(3, 2),
        Queen::new(2, 3),
    ];
    for (i, queen) in queens.iter().enumerate() {
        let threatened: Vec<usize> = (queens.iter().enumerate())
            .filter(|&(j, other)| i != j && queen.threatens(other))
            .map(|(j, _)| j)
            .collect();
        println!("{} threatens {threatened:?}", queen.label());
    }
    let diagonals: Vec<i32> = queens.iter().map(Queen::left_diagonal).collect();
    println!("{diagonals:?} {}", Queen::new(4, 4).right_diagonal());
    let reads: Vec<u32> = queens.iter().map(Queen::read_count).collect();
    println!("{reads:?}");
}
