// A struct expression's fields are evaluated in the order they're written,
// and its object lists them so: `{ c: step(3), b: .., a: .. }`. What
// `..base` fills is after them, a spread of it before; `{:?}` shows them as
// declared, as a derived `Debug` does, and a derived order compares them so.
use std::cell::RefCell;

thread_local! {
    static STEPS: RefCell<Vec<i32>> = const { RefCell::new(Vec::new()) };
}

fn step(n: i32) -> i32 {
    STEPS.with_borrow_mut(|steps| steps.push(n));
    n
}

#[derive(Debug, Clone, Default, PartialEq, PartialOrd)]
struct Point {
    x: i32,
    y: i32,
    label: String,
}

#[derive(Debug)]
enum Shape {
    Rect { width: i32, height: i32 },
}

fn made() -> Point {
    Point { label: step(3).to_string(), y: step(2), x: step(1) }
}

fn moved(p: Point) -> Point {
    Point { y: step(5), x: step(4), ..p }
}

fn defaulted() -> Point {
    Point { label: "d".to_string(), x: step(6), ..Default::default() }
}

fn each(n: i32) -> Vec<Point> {
    let mut points = Vec::new();
    for i in 0..n {
        points.push(Point { y: step(10 + i), x: step(20 + i), label: String::new() });
    }
    points
}

fn main() {
    let p = made();
    println!("{:?}", each(2));
    let q = moved(p.clone());
    let d = defaulted();
    let r = Shape::Rect { height: step(8), width: step(7) };
    let Point { label, x, .. } = q.clone();
    println!("{p:?} {q:?} {d:?} {r:?}");
    println!("{label} {x} {}", p < q);
    println!("{:?}", STEPS.with_borrow(|steps| steps.clone()));
}
