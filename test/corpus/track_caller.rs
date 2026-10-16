// `#[track_caller]` (ADR 0427): a function that tracks its caller and reads
// where it was called is given its caller's location, `{ file, line,
// column }`, and passes it on to one it calls that takes it; one that reads
// nothing of it, std's `Index` of the crate's among them, takes none.
// `Location::caller()` anywhere else, a closure in one too, is where it's
// written; a tracked function as a value is called where it's defined. Its
// file is `test/corpus/track_caller.rs` natively and as JS, as the corpus
// leaves out where the repository is.
use std::ops::Index;
use std::panic::Location;

#[track_caller]
fn here() -> &'static Location<'static> {
    Location::caller()
}

#[track_caller]
fn through() -> u32 {
    here().line()
}

#[track_caller]
fn quiet(x: u32) -> u32 {
    x + 1
}

#[track_caller]
fn in_closure() -> u32 {
    let inner = || Location::caller().line();
    inner()
}

#[track_caller]
fn both() -> (u32, u32) {
    let inner = || Location::caller().line();
    (Location::caller().line(), inner())
}

struct Grid(Vec<u32>);

impl Index<usize> for Grid {
    type Output = u32;
    fn index(&self, i: usize) -> &u32 {
        &self.0[i]
    }
}

fn main() {
    let at = here();
    println!("{} {} {}", at.line(), at.column(), at.file().ends_with("track_caller.rs"));
    println!("{}", through());
    println!("{}", quiet(1));
    println!("{}", in_closure());
    println!("{:?}", both());
    let grid = Grid(vec![4, 5]);
    println!("{}", grid[1]);
    let pointer: fn() -> &'static Location<'static> = here;
    println!("{} {}", pointer().line(), pointer().column());
    let lines: Vec<u32> = [(); 2].iter().map(|_| here().line()).collect();
    println!("{lines:?}");
    println!("{at} {at:?}");
    println!("{:?}", Location::caller());
}
