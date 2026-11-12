// A range's `map(..).collect::<Vec<_>>()` whose closure panics drops each
// item it already made as it unwinds, as std's collect does: an inline
// closure's, which was made `Array.from({ length: n }, f)` and dropped none
// (#1498), a named function's, and items that drop nothing, which keep the
// array form.
struct Cell(u32);
impl Drop for Cell {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn made(n: u32) -> Cell {
    if n == 2 {
        panic!("at 2");
    }
    Cell(n)
}
fn main() {
    let r = std::panic::catch_unwind(|| {
        let w = 3;
        let cells: Vec<Cell> = (0..w)
            .map(|n| {
                if n == 2 {
                    panic!("at 2");
                }
                Cell(n * 0 + 7)
            })
            .collect();
        cells.len()
    });
    println!("{}", r.is_err());
    let r = std::panic::catch_unwind(|| (0..3).map(made).collect::<Vec<_>>().len());
    println!("{}", r.is_err());
    let r = std::panic::catch_unwind(|| {
        (1..4)
            .map(|n| Cell(n * 10))
            .map(|c| if c.0 == 30 { panic!("at 30") } else { c })
            .collect::<Vec<_>>()
            .len()
    });
    println!("{}", r.is_err());
    let squares: Vec<u32> = (0..4).map(|n| n * n).collect();
    let cells: Vec<Cell> = (0..2).map(|n| Cell(n + 100)).collect();
    println!("{squares:?} {}", cells.len());
}
