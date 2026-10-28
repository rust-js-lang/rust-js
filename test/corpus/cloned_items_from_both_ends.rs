// A `cloned()` of what needs no clone, numbers, is its source in JS, as a
// `copied()` is (ADR 0547): an array's items, stepped from its back and
// counted, made here, held, or given to generic code; of strings too.
fn ends<I: DoubleEndedIterator<Item = u32> + ExactSizeIterator>(mut it: I) -> (usize, Option<u32>, Option<u32>) {
    (it.len(), it.next_back(), it.next())
}
fn main() {
    let v = vec![1u32, 2, 3];
    println!("{:?}", v.iter().cloned().rev().collect::<Vec<_>>());
    println!("{} {:?}", v.iter().cloned().len(), v.iter().cloned().next_back());
    println!("{:?}", ends(v.iter().cloned()));
    let mut held = v.iter().cloned();
    held.next();
    println!("{:?} {}", held.next_back(), held.len());
    let s = vec![String::from("a"), String::from("b")];
    println!("{:?}", s.iter().cloned().rev().collect::<Vec<_>>());
}
