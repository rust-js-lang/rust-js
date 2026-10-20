// A `Box<[T]>` is an array: collected into, and cloned item by item where
// its items change in place, as a derived `Clone` clones it (ADR 0530).
#[derive(Clone, Debug)]
struct Holder {
    indices: Box<[usize]>,
    rows: Box<[Vec<u8>]>,
}

fn main() {
    let indices: Box<[usize]> = (0..3).collect();
    let rows: Box<[Vec<u8>]> = vec![vec![1], vec![2, 3]].into_iter().collect();
    let h = Holder { indices, rows };
    let mut copy = h.clone();
    copy.indices[0] = 9;
    copy.rows[1].push(4);
    println!("{h:?} {copy:?}");
}
