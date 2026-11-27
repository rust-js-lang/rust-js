// A `matches!` of a value each branch makes of one variant: MIR's `bool` of
// it is set to a constant and tested, as a drop flag is, so tidy's `flags`
// stage knows it, and takes the branch it runs in place of its test. Where
// that branch returns, what follows the test never runs, and goes with it,
// as the stage ensures (#2622, fuzz seed 12469959).
enum Shape {
    Circle(u32),
    Square(u32),
}

fn side(shape: &Shape) -> u32 {
    match shape {
        Shape::Circle(r) => r * 2,
        Shape::Square(s) => *s,
    }
}

fn square_side(size: u32, doubled: bool) -> Option<u32> {
    if matches!(
        if doubled {
            Shape::Circle(size * 2)
        } else {
            Shape::Circle(size)
        },
        Shape::Circle(_)
    ) {
        return None;
    }
    Some(size)
}

fn main() {
    println!("{:?}", square_side(3, true));
    println!("{:?}", square_side(3, false));
    println!("{}", side(&Shape::Square(4)));
}
