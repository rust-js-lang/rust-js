// A `const { Vec::new() }` is made afresh at each use in Rust: its one JS
// array is copied at each, so changing one changes no other (ADR 0031).
const EMPTY: Vec<i32> = Vec::new();

fn main() {
    let mut first = EMPTY;
    first.push(1);
    let second = EMPTY;
    let grid: [Vec<i32>; 3] = [EMPTY; 3];
    println!("{first:?} {second:?} {grid:?}");
    // One of a value that isn't `Copy` is the one there is.
    let single = [vec![7]; 1];
    println!("{single:?}");
    let mut rows: [Vec<i32>; 2] = [const { Vec::new() }; 2];
    rows[0].push(1);
    println!("{rows:?}");
}
