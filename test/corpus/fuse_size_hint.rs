// `size_hint()` of a `Fuse` no type of the crate's holds: what it steps',
// and none once it's ended (ADR 0473).
fn hint<I: Iterator>(it: I) -> (usize, Option<usize>) {
    let mut fused = it.fuse();
    fused.next();
    fused.size_hint()
}

fn main() {
    let items = [1, 2, 3];
    println!("{:?} {:?}", hint(items.iter()), items.iter().fuse().size_hint());
}
