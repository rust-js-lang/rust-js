// An iterator in a struct's field, lent as a `&mut` to a generic function:
// each call steps the field on.
pub struct P { pub v: std::vec::IntoIter<u32> }
fn first<I: Iterator<Item = u32>>(it: &mut I) -> Option<u32> { it.next() }
pub fn f(p: &mut P) -> Option<u32> { first(&mut p.v) }
fn main() { let mut p = P { v: vec![1, 2].into_iter() }; println!("{:?} {:?} {:?}", f(&mut p), f(&mut p), f(&mut p)); }
