// What a block binds before it branches is seen where the branches meet
// (ADR 0364): the bindings come before the block the branches leave.
use std::hint::black_box;

fn pick(a: u32) -> u32 {
    let x = a + 1;
    let z = x * 3;
    let y;
    if x > 3 {
        y = x * 2;
    } else {
        y = x + 1;
    }
    y + z + x
}

fn main() {
    println!("{}", pick(black_box(5)));
    println!("{}", pick(black_box(1)));
}
