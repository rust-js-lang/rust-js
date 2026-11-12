//! Writes through each `&mut` its library's impls give, as native Rust's
//! land in the box they point into (#1455).
use dep::*;
use std::borrow::BorrowMut;
pub fn main() {
    let mut n = Nested(Box::new(Box::new(1)));
    *n.as_mut() += 4;
    println!("{}", **n.0);
    let mut full = Slot::Full(Box::new(7));
    *full *= 3;
    let mut empty = Slot::Empty(2);
    *empty += 5;
    println!("{} {}", *full, *empty);
    let mut h = Holder {
        point: Box::new(Point { x: 1, y: 2 }),
        more: vec![Box::new(5)],
    };
    h[0] += 9;
    h[1] = 40;
    h[2] += 100;
    let p: &mut Point = h.borrow_mut();
    p.x += 1000;
    println!("{} {} {}", h.point.x, h.point.y, h[0]);
    println!("{:?}", Local.next());
    let mut c = Counter(Box::new(3));
    *c += 4;
    *std::ops::DerefMut::deref_mut(&mut c) *= 2;
    println!("{}", *c);
}
