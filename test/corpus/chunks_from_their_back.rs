// A slice's chunks and windows, the array of them made where they're made,
// stepped from either end as std's are (ADR 0335).
fn main() {
    let v = [1, 2, 3, 4, 5];
    let mut c = v.chunks(2);
    println!("{:?} {:?} {:?}", c.next_back(), c.next(), c.len());
    let mut w = v.windows(3);
    println!("{:?} {:?} {:?}", w.next_back(), w.next_back(), w.collect::<Vec<_>>());
    println!(
        "{:?}",
        v.chunks(2).rev().map(|c| c.iter().sum::<i32>()).collect::<Vec<_>>()
    );
    println!("{:?} {:?}", v.windows(2).rposition(|w| w[0] < 3), v.chunks(3).last());
    let mut m = [1, 2, 3, 4, 5];
    for c in m.chunks_mut(2).rev() {
        c[0] *= 10;
    }
    println!("{:?}", m);
    println!("{:?}", v.chunks(2).nth_back(1));
    let e: Vec<&[i32]> = [0i32; 0].chunks(2).collect();
    println!("{:?}", e);
}
