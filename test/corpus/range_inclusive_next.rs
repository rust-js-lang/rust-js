// `next()` of a `RangeInclusive` given as a `&mut`: it steps to its end,
// then gives `None`, as an empty one does.
pub fn f(r: &mut std::ops::RangeInclusive<u32>) -> Option<u32> { r.next() }
fn main() { let mut r = 1..=3; for _ in 0..5 { println!("{:?} {:?}", f(&mut r), r); } let mut e = 5..=4; println!("{:?}", f(&mut e)); }
