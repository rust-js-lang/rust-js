// `mem::discriminant(&e)` is `e`'s variant's discriminant, as rustc numbers
// it, a number: equal for one variant whatever its fields, shown as std
// shows it, a map's key (ADR 0393).
use std::collections::HashMap;
use std::mem::{discriminant, Discriminant};

#[derive(Debug)]
enum Shape {
    Circle(f64),
    Square { side: f64 },
    Empty,
}

#[allow(dead_code)]
enum Code {
    Low = 3,
    High = 10,
}

const EMPTY: Discriminant<Shape> = discriminant(&Shape::Empty);
// A static's is its value, which no MIR of its own gives.
static SQUARE: Discriminant<Shape> = discriminant(&Shape::Square { side: 0.0 });

fn same_kind(a: &Shape, b: &Shape) -> bool {
    discriminant(a) == discriminant(b)
}

fn main() {
    let shapes = vec![Shape::Circle(1.0), Shape::Square { side: 2.0 }, Shape::Circle(3.0), Shape::Empty];
    println!("{} {}", same_kind(&shapes[0], &shapes[2]), same_kind(&shapes[0], &shapes[1]));
    let mut counts: HashMap<Discriminant<Shape>, u32> = HashMap::new();
    for shape in &shapes {
        *counts.entry(discriminant(shape)).or_insert(0) += 1;
    }
    println!("{:?} {:?}", counts.get(&discriminant(&Shape::Circle(0.0))), counts.get(&EMPTY));
    println!("{:?}", discriminant(&Shape::Square { side: 1.0 }));
    println!("{:?} {:?}", discriminant(&Code::Low), discriminant(&Code::High));
    println!("{} {}", discriminant(&shapes[3]) == EMPTY, discriminant(&shapes[1]) == SQUARE);
    println!("{:?}", shapes[1]);
}
