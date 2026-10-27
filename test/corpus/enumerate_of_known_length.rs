// An `enumerate()` of what knows its length knows its own, made in place
// over a parameter, a `map` of one, held and stepped from its back (ADRs
// 0456, 0528; #382).

fn counted<I: ExactSizeIterator<Item = u8>>(it: I) -> usize {
    it.enumerate().len()
}
fn mapped<I: ExactSizeIterator<Item = u8>>(it: I) -> usize {
    it.map(|x| x + 1).enumerate().len()
}
fn backs<I: DoubleEndedIterator<Item = u8> + ExactSizeIterator>(it: I) -> (usize, Option<(usize, u8)>) {
    let mut e = it.enumerate();
    let back = e.next_back();
    (e.len(), back)
}
fn main() {
    let v = vec![1u8, 2, 3];
    println!("{} {}", counted(v.iter().copied()), mapped([4u8, 5].into_iter()));
    println!("{:?}", backs(v.iter().copied()));
    let e = v.iter().enumerate();
    println!("{}", e.len());
    let r = v.iter().rev().enumerate();
    println!("{}", r.len());
}
