// `{:#?}` of the crate's derived `Debug`s, with nothing else asking for
// `alternate()`: the placeholder alone says the crate shows them pretty
// (ADR 0137).

#[derive(Debug)]
struct Point {
    x: u8,
    tags: Vec<&'static str>,
}

#[derive(Debug)]
enum Shape {
    Dot(Point),
    Empty,
}

fn main() {
    let shape = Shape::Dot(Point { x: 1, tags: vec!["a", "b"] });
    println!("{:#?}", shape);
    println!("{:?} {:#?}", Shape::Empty, Shape::Empty);
}
