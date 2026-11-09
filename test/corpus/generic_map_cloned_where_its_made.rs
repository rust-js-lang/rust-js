//@ library-differs
// Generic code that cycles the `Map<I, F>` it makes in place, bound by
// `Clone`: what its type stands for is cloned each round, whatever its
// bounds say, so it's made cloned apart, each round from where it started
// (ADR 0525, #955).
fn cycled<I: Iterator<Item = i32> + Clone, F: FnMut(i32) -> i32 + Clone>(items: I, f: F) -> Vec<i32> {
    items.map(f).cycle().take(5).collect()
}

fn main() {
    println!("{:?}", cycled(1..3, |x| x * x));
    println!("{:?}", cycled(vec![4, 5, 6].into_iter(), |x| -x));
}
