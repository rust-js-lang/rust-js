// An or-pattern over `Option`s or enums, `(None, _) | (_, 1)`, tests each
// alternative's parts in the order written (#674): `x === 0 && n === 0`
// of `(Some(0), 0)`, not MIR's `n === 0 && x === 0`, its `x != null`
// said by `x === 0`, and the chain as written, `x == null || n === 1`.
// Tests of one part and of another move past each other; a part's own
// tests never do, `x != null` before `x[0]`, a slice's `length` before
// `xs[0]`, an enum's `TAG` before its field. `paired` and `shaped` keep
// the tree, whose chain would read more tests. Each input takes the arm
// native Rust takes.
pub fn optional(x: Option<u32>, n: u32) -> u32 {
    match (x, n) {
        (Some(0), 0) => 1,
        (None, _) | (_, 1) => 2,
        (Some(2), 2) | (Some(3), 3) => 3,
        _ => 4,
    }
}

pub fn measured(x: f64, n: u8) -> u8 {
    match (x, n) {
        (_, 0) => 1,
        (1.0, _) | (_, 1) => 2,
        (2.0, 2) | (_, 3) => 3,
        _ => 4,
    }
}

pub fn paired(x: Option<(u8, u8)>, n: u8) -> u8 {
    match (x, n) {
        (Some((0, 0)), 0) => 1,
        (None, _) | (_, 1) => 2,
        (Some((2, _)), 2) | (Some((_, 3)), 3) => 3,
        _ => 4,
    }
}

pub enum Shape {
    Circle(u32),
    Rect(u32, u32),
}

pub fn shaped(shape: &Shape, k: u32) -> u32 {
    match (shape, k) {
        (Shape::Circle(0), 0) => 1,
        (Shape::Circle(1), _) | (_, 1) => 2,
        (Shape::Rect(2, 2), _) | (Shape::Circle(2), 2) => 3,
        _ => 4,
    }
}

pub fn named(text: Option<&str>, n: u32) -> u32 {
    match (text, n) {
        (Some("a"), 0) => 1,
        (None, _) | (_, 1) => 2,
        (Some("b"), 2) | (Some("c"), 3) => 3,
        _ => 4,
    }
}

pub fn sliced(xs: &[u32], n: u32) -> u32 {
    match (xs, n) {
        ([0], 0) => 1,
        ([], _) | (_, 1) => 2,
        ([2, _], 2) | ([_, 3], 3) => 3,
        _ => 4,
    }
}

fn main() {
    let mut out = Vec::new();
    for x in [None, Some(0), Some(1), Some(2), Some(3)] {
        for n in 0..5 {
            out.push(optional(x, n));
        }
    }
    for x in [0.0, 1.0, 2.0, -1.0, f64::NAN, f64::INFINITY] {
        for n in 0..5 {
            out.push(u32::from(measured(x, n)));
        }
    }
    for x in [None, Some((0, 0)), Some((2, 0)), Some((0, 3)), Some((2, 3)), Some((1, 1))] {
        for n in 0..5 {
            out.push(u32::from(paired(x, n)));
        }
    }
    for a in 0..4 {
        for k in 0..4 {
            out.push(shaped(&Shape::Circle(a), k));
            for b in 0..4 {
                out.push(shaped(&Shape::Rect(a, b), k));
            }
        }
    }
    for text in [None, Some("a"), Some("b"), Some("c"), Some("")] {
        for n in 0..5 {
            out.push(named(text, n));
        }
    }
    let lists: [&[u32]; 7] = [&[], &[0], &[2], &[2, 3], &[0, 3], &[2, 0, 0], &[1, 1]];
    for xs in lists {
        for n in 0..5 {
            out.push(sliced(xs, n));
        }
    }
    println!("{out:?}");
}
