// `rchunks_exact` is an array of its chunks, as `chunks_exact` is, which
// keeps what's left at the start, its `remainder()`, stepped or not.
fn main() {
    let a = [1, 2, 3, 4, 5, 6, 7, 8];
    println!("{:?}", a.rchunks_exact(3).remainder());
    let mut it = a.rchunks_exact(3);
    println!("{:?} {:?}", it.next(), it.remainder());
    let mut b = [1, 2, 3, 4, 5];
    let mut m = b.rchunks_exact_mut(2);
    if let Some(last) = m.next() {
        last[0] = 0;
    }
    println!("{:?}", m.into_remainder());
    println!("{b:?} {:?}", a.rchunks(3).collect::<Vec<_>>());
}
