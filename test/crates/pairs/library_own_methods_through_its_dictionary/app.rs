//! Each called as the library's own, not std's: its `max` keeps the first
//! of equals, its `!=` is its `ne`, and the clone its default makes is the
//! program's to change, and its writer's `Err` is given back.
use dep::{A, B, Full, Holder, Lists, Picky, S, Tr};
use std::fmt::Write;

pub fn main() {
    println!("{:?}", Picky(1, 'a').max(Picky(1, 'b')));
    println!("{:?}", std::cmp::max(Picky(1, 'a'), Picky(1, 'b')));
    println!("{}", A(1) != B(2));
    println!("{}", &A(3) != &B(3));
    let h = Holder::<Lists> { item: vec![1] };
    let mut c = S::copied(&h);
    c.item.push(2);
    println!("{:?} {:?}", h.item, c.item);
    let mut f = Full(5);
    println!("{:?} {}", f.write_str("abc"), f.0);
    println!("{:?} {}", f.write_str("abc"), f.0);
}
