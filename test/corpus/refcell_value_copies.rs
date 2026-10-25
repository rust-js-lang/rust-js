// A `RefCell` of a `Copy` struct is the struct (ADR 0863), copied where
// it's made of one, as a `Copy` value changed in place is (ADR 0052): two
// cells made of one are two values, each changed apart.

use std::cell::RefCell;

#[derive(Clone, Copy, Debug)]
struct Point {
    x: i32,
    y: i32,
}

fn main() {
    let p = Point { x: 1, y: 2 };
    let a = RefCell::new(p);
    let b = RefCell::new(p);
    a.borrow_mut().x = 10;
    b.borrow_mut().y = 20;
    let copied = *a.borrow();
    a.borrow_mut().y = 30;
    println!("{:?} {:?} {:?} {:?}", a, b, copied, p);
    let moved = a.into_inner();
    println!("{} {}", moved.x + moved.y, b.into_inner().y);
}
