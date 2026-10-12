// A named constant a function gives is a copy of it: what its caller
// changes is its own, the constant as it was.
#[derive(Debug, Clone, Copy)]
enum Shape {
    Circle(i32),
    Dot,
}

const SHAPE: Shape = Shape::Circle(1);

fn shape() -> Shape {
    SHAPE
}

fn main() {
    let mut s = shape();
    if let Shape::Circle(r) = &mut s {
        *r = 5;
    }
    println!("{:?} {:?} {:?} {:?}", s, shape(), SHAPE, Shape::Dot);
}
