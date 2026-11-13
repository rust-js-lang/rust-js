// A `Vec` given as a side of a chain or a zip a `Vec` is grown by, in a
// crate that reads a `Vec`'s capacity, whose other side isn't `TrustedLen`,
// so std grows it by the sum of their hints, or the fewer: the `Vec`'s JS
// is the array itself, which the runtime's chain stepped as JS's own array
// iterator, with no hint, so it grew by the other side's alone, 18 where
// std's capacity is 14 (#1658).
fn capacity<T>(v: Vec<T>) -> usize {
    v.capacity()
}

fn main() {
    let v: Vec<u32> = (0..10).collect();
    let c: Vec<u32> = v.into_iter().skip(1).chain(vec![1, 2, 3, 4, 5]).collect();
    println!("{c:?} {}", c.capacity());
    let a = [1u32, 2, 3, 4, 5, 6];
    println!("{}", capacity(a.iter().copied().skip(1).chain(vec![7, 8, 9]).collect()));
    println!("{}", capacity(vec![7u32, 8].into_iter().chain(a.iter().copied().skip(2)).collect()));
    println!("{}", capacity(a.iter().copied().skip(1).zip(vec![1u32, 2, 3]).collect()));
    println!("{}", capacity(Vec::<u32>::new().into_iter().chain(a.iter().copied().skip(5)).collect()));
}
