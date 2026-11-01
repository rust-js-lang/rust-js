// A `Vec`'s `extend` of an iterator over an array's items, an index loop
// that pushes each (ADR 1042, the user's decision): of a slice's, a
// `Vec`'s, a list a call makes, read once, onto a field, and of `Copy`
// structs, each copied, as Rust copies them. A slice or a `Vec` given
// whole is stepped by its own iteration, and a heap's `extend` of a
// slice's iterator is of the slice.
use std::collections::BinaryHeap;

#[derive(Clone, Copy, Debug)]
struct Point {
    x: i32,
}

struct Holder {
    items: Vec<u32>,
}

fn made(n: u32) -> Vec<u32> {
    println!("made {n}");
    (0..n).collect()
}

fn main() {
    let xs = [1u32, 2, 3];
    let mut out = vec![0];
    out.extend(&xs);
    out.extend(xs.iter().rev().rev());
    out.extend(xs.iter());
    out.extend(vec![7, 8]);
    out.extend(made(2).iter());
    let mut holder = Holder { items: Vec::new() };
    holder.items.extend(out[1..3].iter());
    println!("{out:?} {:?}", holder.items);

    let points = [Point { x: 1 }, Point { x: 2 }];
    let mut copied: Vec<Point> = Vec::new();
    copied.extend(points.iter());
    copied[0].x = 9;
    println!("{:?} {:?}", points, copied);

    let empty: &[u32] = &[];
    out.extend(empty.iter());
    println!("{}", out.len());

    let mut heap = BinaryHeap::new();
    heap.extend(xs.iter().copied());
    println!("{:?}", heap.into_sorted_vec());
}
