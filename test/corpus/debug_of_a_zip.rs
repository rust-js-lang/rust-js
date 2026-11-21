// `{:?}` of a `Zip` of two `TrustedRandomAccessNoCoerce` sides, slices'
// iterators: std's `ZipFmt` shows it `Zip` alone, never what its sides hold,
// by their types, however far it was stepped (#2252).
fn main() {
    let mut pairs = [1, 2].iter().zip([3, 4].iter());
    println!("{:?}", pairs);
    pairs.next();
    println!("{:?} {:#?}", pairs, pairs);
    let v = vec![5, 6];
    let skipped = v.iter().zip(v.iter().skip(1));
    println!("{skipped:?}");
    println!("{}", skipped.count());
    let chunks = v.chunks(1).zip(v.iter());
    println!("{chunks:?}");
}
