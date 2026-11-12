//@ library-differs
// `{:#?}` of a type whose hand-written `Debug` shows values it makes,
// not its fields: each is shown pretty too, as Rust's `Formatter` passes
// its options down to whatever a writer shows (ADR 0137). The type shown
// pretty and its fields' types aren't the only ones that take options.
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
        let cells: Vec<Cell> = (0..self.width).map(|n| Cell { n: n * 100 }).collect();
        f.debug_list().entries(cells.iter()).finish()
    }
}

fn main() {
    let grid = Grid { width: 2 };
    println!("{:#?}", grid);
    println!("{:?}", grid);
}
