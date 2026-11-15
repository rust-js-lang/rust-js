// Variants of the same fields are copied, cloned and compared alike, one
// test of them all (#1821): `Shape`'s `Circle` and `Square` each hold an
// `f64` and a `Vec<u32>`, its `Line` two `Vec<u32>`s, its `Dot` nothing;
// `Nest`'s four variants each hold the `Shape` before them. Each cloned out
// of a `RefCell`, changed, and compared with `cmp`, `partial_cmp` and `==`,
// as the derives do it, variant first, then the fields in turn.
use std::cell::RefCell;
use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
enum Shape {
    Dot,
    Circle(f64, Vec<u32>),
    Square(f64, Vec<u32>),
    Line(Vec<u32>, Vec<u32>),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Nest {
    Leaf,
    First(Vec<u32>),
    Second(Vec<u32>),
    Third(Vec<u32>),
    Fourth(Box<Nest>),
}

fn main() {
    let shapes = RefCell::new(vec![
        Shape::Dot,
        Shape::Circle(1.5, vec![1]),
        Shape::Square(1.5, vec![1]),
        Shape::Line(vec![2], vec![3]),
    ]);
    let mut copies: Vec<Shape> = shapes.borrow().iter().cloned().collect();
    for copy in &mut copies {
        match copy {
            Shape::Circle(_, v) | Shape::Square(_, v) => v.push(9),
            Shape::Line(a, b) => {
                a.push(7);
                b.clear();
            }
            Shape::Dot => {}
        }
    }
    println!("{:?}", shapes.borrow());
    println!("{copies:?}");
    for (a, b) in shapes.borrow().iter().zip(&copies) {
        println!("{:?} {}", a.partial_cmp(b), a == b);
    }
    let circle = Shape::Circle(f64::NAN, vec![1]);
    println!("{:?} {:?}", circle.partial_cmp(&circle.clone()), Shape::Dot.partial_cmp(&Shape::Line(vec![], vec![])));

    let nests = RefCell::new(Nest::Fourth(Box::new(Nest::Second(vec![4]))));
    let mut nest = nests.borrow().clone();
    if let Nest::Fourth(inner) = &mut nest {
        if let Nest::Second(v) = inner.as_mut() {
            v.push(5);
        }
    }
    println!("{:?} {:?}", nests.borrow(), nest);
    let mut all = vec![
        Nest::Third(vec![1]),
        Nest::First(vec![2]),
        Nest::Leaf,
        Nest::First(vec![1, 1]),
        nest.clone(),
        Nest::Second(vec![]),
        nests.borrow().clone(),
    ];
    all.sort();
    println!("{all:?}");
    println!(
        "{:?} {:?} {:?}",
        Nest::First(vec![3]).cmp(&Nest::Second(vec![1])),
        Nest::Third(vec![2]).cmp(&Nest::Third(vec![1, 9])),
        nest.cmp(&nests.borrow()) == Ordering::Greater
    );
}
