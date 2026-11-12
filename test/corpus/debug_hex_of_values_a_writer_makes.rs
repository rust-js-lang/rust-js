// `{:x?}` of a type whose hand-written `Debug` shows values it makes,
// not its fields: their integers are in hex too, as Rust's `Formatter`
// passes the flag down to whatever a writer shows (ADR 0559). The type
// shown in hex and its fields' types aren't the only ones that take it.
use std::fmt;

#[derive(Debug)]
struct Cell {
    n: u32,
}

struct Grid {
    width: u32,
}

impl fmt::Debug for Grid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let cells: Vec<Cell> = (0..self.width).map(|n| Cell { n: n * 100 + 7 }).collect();
        f.debug_list().entries(cells.iter()).finish()
    }
}

fn main() {
    let grid = Grid { width: 3 };
    println!("{:x?}", grid);
    println!("{:X?}", grid);
    println!("{:?}", grid);
}
