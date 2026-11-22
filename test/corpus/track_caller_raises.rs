//@ run-fail: index out of bounds: the len is 2 but the index is 9
// A `#[track_caller]` function that raises a panic itself, `panic!`'s,
// `unwrap`'s, a `Vec`'s index, a division's, takes its caller's location
// and names it, as rustc places each of its panics where it was called
// (#2350): a caught one, and the crate's `Index` of a `Vec`, whose trait
// tracks its caller, through generic code's dictionary, last.
use std::ops::Index;
use std::panic;

#[track_caller]
fn check(n: u32) -> u32 {
    if n > 3 {
        panic!("too big: {n}");
    }
    n
}

#[track_caller]
fn share(total: u32, n: u32) -> u32 {
    total / n
}

#[track_caller]
fn first(v: &[u32]) -> u32 {
    *v.first().unwrap()
}

#[track_caller]
fn twice(n: u32) -> u32 {
    check(n) + check(n * 2)
}

struct Grid(Vec<u32>);

impl Index<usize> for Grid {
    type Output = u32;
    fn index(&self, i: usize) -> &u32 {
        &self.0[i]
    }
}

fn get<G: Index<usize, Output = u32>>(g: &G, i: usize) -> u32 {
    g[i]
}

fn main() {
    println!("{} {} {} {}", check(1), share(6, 2), first(&[4]), twice(1));
    let caught = panic::catch_unwind(|| twice(std::hint::black_box(3)));
    println!("caught: {}", caught.is_err());
    let grid = Grid(vec![4, 5]);
    println!("{} {}", grid[1], get(&grid, 0));
    println!("{}", get(&grid, std::hint::black_box(9)));
}
