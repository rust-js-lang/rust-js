//@ native: wasm32
// `cycle()`'s `size_hint()` is std's: none of a source that's empty, as
// its hint says, `(0, Some(0))`; else without end, `(usize::MAX, None)`;
// of a range's, by its step, and of a `Vec`'s or an array's items (#1179).
fn main() {
    println!("{:?}", (1..10).cycle().size_hint());
    println!("{:?}", (1..1).cycle().size_hint());
    println!("{:?}", (1..=3).cycle().size_hint());
    println!("{:?}", vec![1, 2].into_iter().cycle().size_hint());
    println!("{:?}", Vec::<u8>::new().into_iter().cycle().size_hint());
    println!("{:?}", [1, 2].iter().cycle().size_hint());
    let mut c = (1..3).cycle();
    c.next();
    c.next();
    c.next();
    println!("{:?}", c.size_hint());
    println!("{:?}", (1..3).cycle().take(5).size_hint());
    let all: Vec<u32> = (1..3).cycle().take(5).collect();
    println!("{all:?} {}", all.capacity() >= 5);
}
