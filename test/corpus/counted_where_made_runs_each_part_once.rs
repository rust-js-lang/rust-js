// An iterator made in place is counted from what it's made of (ADR 0986),
// and every part it's made of runs once, in Rust's order. `eq` of two
// `TrustedLen` iterators asks their lengths before it steps them
// (ADR 0992), and a length read before the iterator is made of the same
// text doesn't run a range's bound again (#1483). A `len()` or a
// `size_hint()` of one, which isn't made after, runs what its count
// leaves unread, `repeat_n(x, n)`'s `x`, and a range's bounds start first
// (#1485). A count read through a call, `*held.as_ref().unwrap()`, is
// read once.
fn made(n: usize) -> usize {
    println!("made {n}");
    n
}

fn main() {
    let zipped = (0..made(2)).zip(made(1)..made(4)).eq((0..2).zip(1..3));
    println!("zipped eq: {zipped}");
    let chained = (0..made(2)).chain(made(1)..made(4)).eq(0..5);
    println!("chained eq: {chained}");
    let unequal = (made(0)..made(3)).eq(0..2);
    println!("unequal eq: {unequal}");

    let hint = std::iter::repeat_n(made(4), 3).size_hint();
    println!("repeat_n hint: {hint:?}");
    let taken = std::iter::repeat(made(5)).take(2).len();
    println!("taken len: {taken}");
    let stepped = (made(6)..made(11)).step_by(2).len();
    println!("stepped len: {stepped}");
    let zip_len = std::iter::repeat_n(made(7), 3).zip(made(0)..made(8)).len();
    println!("zip len: {zip_len}");
    let pair_hint = (0..made(2)).zip(made(1)..made(4)).size_hint();
    println!("zip hint: {pair_hint:?}");

    let held = Some(3usize);
    let repeated = std::iter::repeat_n(1u8, *held.as_ref().unwrap()).len();
    let mapped = std::iter::repeat_n(1u8, *held.as_ref().unwrap())
        .map(|x| x + 1)
        .len();
    let through_take = std::iter::repeat(1u8).take(*held.as_ref().unwrap()).len();
    let step = (0..10).step_by(*Some(2).as_ref().unwrap()).len();
    println!("read once: {repeated} {mapped} {through_take} {step}");
}
