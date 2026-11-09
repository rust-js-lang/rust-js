// Generic code's chain of its own parameters is each instance its callers
// give it, where they're all seen: one generic function's cloned chain
// isn't another's stepped from its back, though rustc names both
// `Chain<I, I>` and their bounds agree, unless a caller gives both the
// same instance, which is refused (#1119; ADR 0525).
fn chained<I: DoubleEndedIterator<Item = u32>>(a: I, b: I) -> Option<u32> {
    a.chain(b).map(|x| x + 1).next_back()
}

fn summed<I: DoubleEndedIterator<Item = i32>>(a: I, b: I) -> Option<i32> {
    a.chain(b).map(|x| x * 10).next_back()
}

fn chain_cycled<J: Iterator<Item = i32> + Clone>(a: J, b: J) -> Vec<i32> {
    a.chain(b).cycle().take(5).collect()
}

fn main() {
    println!("{:?}", chained(1..3, 4..6));
    println!("{:?}", chained(1..3, 4..4));
    println!("{:?}", summed(vec![1, 2].into_iter(), vec![4].into_iter()));
    println!("{:?}", summed(vec![1].into_iter(), Vec::new().into_iter()));
    println!("{:?}", chain_cycled(1..3, 4..5));
}
