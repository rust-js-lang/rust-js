// Oracle C's own case (ADR 1204): functions taking what JS makes, each
// made from the records, given plain and as counting proxies, which must
// give alike and what native Rust gives: a struct by value and by
// reference, one of a nullable field, a tuple struct, a unit struct, and
// each form of enum.
pub struct Point {
    pub x: u32,
    pub y: u32,
    pub max_len: u8,
    pub label: Option<String>,
}

pub struct Tagged {
    pub id: u8,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub parent: Option<u8>,
}

pub struct Pair(pub u16, pub bool);

pub struct Marker;

pub enum Shape {
    Empty,
    Circle { radius: u32 },
    Square(u8),
}

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Size {
    Named(String),
    Exact(u32),
}

pub fn spread(a: Point, b: &Point) -> u64 {
    let label = a.label.map_or(0, |l| l.len().min(usize::from(a.max_len)) as u64);
    u64::from(a.x.abs_diff(b.x)) + u64::from(a.y.abs_diff(b.y)) + label
}

pub fn depth(t: &Tagged) -> u16 {
    u16::from(t.id) + t.parent.map_or(0, u16::from)
}

pub fn summed(p: Pair) -> u32 {
    u32::from(p.0) + u32::from(p.1)
}

pub fn marked(_m: Marker, n: u8) -> u8 {
    n.wrapping_add(1)
}

pub fn area(s: &Shape) -> u64 {
    match s {
        Shape::Empty => 0,
        Shape::Circle { radius } => u64::from(*radius) * u64::from(*radius) * 3,
        Shape::Square(side) => u64::from(*side) * u64::from(*side),
    }
}

pub fn measured(s: Size) -> u32 {
    match s {
        Size::Named(name) => name.chars().count() as u32,
        Size::Exact(n) => n,
    }
}

pub fn counted(points: &[Point]) -> usize {
    points.iter().filter(|p| p.x > p.y).count()
}

// A field Rust reads once, kept in a `let` its uses read (ADR 1022's
// shape): read in place at each use, a getter's would run each time.
pub fn kept(p: &Point) -> u32 {
    let x = p.x;
    if x > 3 { x * 2 } else { x + p.y }
}
