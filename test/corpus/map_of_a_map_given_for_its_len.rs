// A map of a map, of `repeat_n`, a range, a `Vec` or a slice, given to
// generic code that asks its `len()`, is counted as its maps' source is: its
// inner map, JS's own `.map(..)`, was taken for std's compiled struct and
// asked a `len` it hasn't, throwing (#1491).
fn n<I: ExactSizeIterator>(it: I) -> usize {
    it.len()
}

fn main() {
    println!("{}", n(std::iter::repeat_n(0u8, 3).map(|x| x + 1)));
    let k = Some(2usize);
    println!(
        "{}",
        n(std::iter::repeat_n(0u8, *k.as_ref().unwrap()).map(|x| x + 1).map(|x| x))
    );
    println!("{}", n(vec![1, 2, 3].into_iter().map(|x| x + 1).map(|x| x * 2)));
    println!("{}", n((0..4u32).map(|x| x + 1).map(|x| x * 2).map(|x| x)));
    println!("{}", n([1u8, 2].iter().map(|x| x + 1).map(|x| x * 2)));
}
