// A `copied()` of a type parameter, std's adapter the recognizers make,
// stepped from its back or counted, asks what its impl asks of what it
// holds, the parameter's `DoubleEndedIterator` or `ExactSizeIterator`, also
// through a `&mut` (ADR 0542; #382).

fn last_copied<'a, I: DoubleEndedIterator<Item = &'a u8>>(it: I) -> Option<u8> {
    it.copied().next_back()
}
fn last_through<'a, R: DoubleEndedIterator<Item = &'a u8>>(r: &mut R) -> Option<u8> {
    r.copied().next_back()
}
fn counted<'a, I: ExactSizeIterator<Item = &'a u8>>(it: I) -> usize {
    it.copied().len()
}
fn main() {
    let v = vec![1u8, 2, 3];
    println!("{:?}", last_copied(v.iter()));
    let mut it = v.iter();
    println!("{:?} {:?}", last_through(&mut it), it.next());
    println!("{}", counted(v.iter()));
}
