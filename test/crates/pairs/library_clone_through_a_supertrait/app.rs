use dep::{Point, Twice, duped};

pub fn main() {
    println!("{}", duped(&Point { x: 1 }, |p| p.x += 1));
    let a = Point { x: 1 };
    let mut b = a.again();
    b.x += 5;
    println!("{a:?} {b:?}");
}
