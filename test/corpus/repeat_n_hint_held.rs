// `repeat_n(x, n)`'s hint is exact, what it has left both ways, and so is
// `repeat(x).take(n)`'s, where an adapter holds it: a `peekable()`, before
// and after a peek, a `fuse()`, a `zip`, a `chain`, a `skip`, a `map` and
// an `enumerate()`, and what a `Vec`'s `collect` reserves by it; and
// `from_fn`'s, std's `(0, None)`, `successors`', `(1, None)`, and
// `repeat_with(f).take(n)`'s (#1357). Never `repeat(x)`'s alone, whose
// `usize::MAX` is wasm32's, rust-js's target, where the host's native
// run's is wider.
use std::iter::{from_fn, repeat, repeat_n};

fn main() {
    let mut held = repeat_n(2u32, 4).peekable();
    println!("{:?}", held.size_hint());
    println!("{:?}", held.peek());
    println!("{:?}", held.size_hint());
    held.next();
    println!("{:?}", held.size_hint());

    let twos = repeat(2u32);
    println!("{:?}", twos.take(4).peekable().size_hint());
    println!("{:?}", repeat_n('x', 3).fuse().peekable().size_hint());
    println!("{:?}", repeat_n(1u8, 4).zip(0..3).peekable().size_hint());
    println!("{:?}", repeat_n(1u8, 4).chain(repeat_n(3, 2)).peekable().size_hint());
    println!("{:?}", repeat_n(1u8, 4).skip(1).peekable().size_hint());
    println!("{:?}", repeat_n(1u8, 4).map(|n| n + 1).peekable().size_hint());
    println!("{:?}", repeat_n(1u8, 4).enumerate().peekable().size_hint());
    println!("{:?}", repeat_n(String::from("a"), 0).peekable().size_hint());

    let mut n = 0;
    let counted = from_fn(|| {
        n += 1;
        (n < 3).then_some(n)
    });
    println!("{:?}", counted.peekable().size_hint());
    let halves = std::iter::successors(Some(8u32), |n| (*n > 1).then_some(n / 2));
    println!("{:?}", halves.peekable().size_hint());
    let ones = std::iter::repeat_with(|| 1u8);
    println!("{:?}", ones.take(3).peekable().size_hint());

    // What a collect reserves by it.
    let made: Vec<u32> = repeat_n(7, 5).peekable().collect();
    println!("{made:?} {}", made.capacity());
    let sevens = repeat(7);
    let made: Vec<u32> = sevens.take(3).peekable().collect();
    println!("{made:?} {}", made.capacity());
    let made: String = repeat_n('z', 6).peekable().collect();
    println!("{made}");
}
