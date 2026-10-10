// A `&mut` to a number given to a function is a box the caller copies
// back once it returns (ADR 0074); one given on is the same box.
fn bump(n: &mut u32, by: u32) -> u32 {
    *n += by;
    *n * 2
}

fn twice(count: &mut u32) {
    bump(count, 1);
    bump(count, 1);
}

fn main() {
    let mut x = 3;
    let doubled = bump(&mut x, 2);
    twice(&mut x);
    println!("{x} {doubled}");
}
