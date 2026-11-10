// A boxed iterator, `Box<I>` of a sized `I`, stepped by what std's `impl
// Iterator for Box<I>` passes on to `I`, `next()`, `size_hint()`, `nth`,
// `next_back()`, `last()`, and by the trait's defaults of the box's value,
// `take(n)`, `map(f)`, `sum()`, a `for`, and of a `&mut` to the box: a
// `Box<I>` is its `I` in JS, so each is `I`'s, where `next()` threw
// `it.next is not a function` (#1278).
struct Countdown(u32);

impl Iterator for Countdown {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.0.checked_sub(1).inspect(|n| self.0 = *n)
    }
}

fn first_two<I: Iterator<Item = u8>>(mut it: Box<I>) -> (Option<u8>, Option<u8>) {
    (it.next(), it.next())
}

// A `&mut` to a box a function's given, a variable's: what it holds.
fn stepped(boxed: &mut Box<std::ops::Range<i32>>) -> (Option<i32>, Option<i32>) {
    (boxed.next(), boxed.next_back())
}

fn main() {
    let mut it = Box::new(vec![1u8, 2].into_iter().map(|x| x + 1));
    while let Some(n) = it.next() {
        println!("got {n}");
    }
    let mut own = Box::new(Countdown(3));
    println!("{:?} {:?}", own.next(), own.size_hint());
    println!("{:?}", own.nth(0));
    println!("{:?}", own.next());
    let mut ranged = Box::new(0..5);
    println!("{:?} {:?}", ranged.next(), ranged.next_back());
    let taken: Vec<_> = (&mut ranged).take(2).collect();
    println!("{taken:?} {:?}", ranged.next());
    println!("{:?} {:?}", stepped(&mut ranged), ranged);
    println!("{:?}", first_two(Box::new([7u8, 8, 9].into_iter())));
    let mut chars = Box::new("hey".chars());
    println!("{:?} {:?}", chars.next(), chars.last());
    let boxed = Box::new(0..5);
    println!("{:?}", boxed.take(2).collect::<Vec<_>>());
    let boxed = Box::new([1, 2, 3].into_iter());
    println!("{:?}", boxed.map(|x| x * 10).collect::<Vec<_>>());
    for c in Box::new("ab".chars()) {
        println!("{c}");
    }
    println!("{}", Box::new(1..4).sum::<i32>());
    let mut dynamic: Box<dyn Iterator<Item = u32>> = Box::new(Countdown(2));
    println!("{:?} {:?}", dynamic.next(), dynamic.next());
}
