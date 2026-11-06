// How many of a zip's first side's items are made, its closure run, where
// its second side ends first, as std's specializations decide: of two
// sides std reads by index, a slice's map and an array, the shorter's count
// alone, by `collect` and `next()` alike (ADR 0973); of two `TrustedLen`
// sides, an inclusive range's or a chain's map, `fold` and `collect` the
// shorter's count, `next()` one more (#654); of a side neither, a filter's,
// one more by each (#115).
fn main() {
    let v = [1, 2, 3];
    let z: Vec<_> = v
        .iter()
        .map(|x| {
            println!("a{x}");
            x
        })
        .zip([10, 20])
        .collect();
    println!("{z:?}");
    let w = vec![1u32, 2, 3, 4];
    let f: u32 = (1..=4u32)
        .map(|x| {
            println!("b{x}");
            x
        })
        .zip(vec![1u32, 2])
        .fold(0, |n, (x, y)| n + x * y);
    println!("{f}");
    let c: Vec<_> = w
        .iter()
        .chain(w.iter())
        .map(|x| {
            println!("c{x}");
            x
        })
        .zip([1, 2])
        .collect();
    println!("{c:?}");
    let mut n = (1..=4u32)
        .map(|x| {
            println!("d{x}");
            x
        })
        .zip([1, 2]);
    println!("{:?} {:?} {:?}", n.next(), n.next(), n.next());
    let g: Vec<_> = w
        .iter()
        .filter(|x| {
            println!("e{x}");
            **x > 0
        })
        .zip([1, 2])
        .collect();
    println!("{g:?}");
}
