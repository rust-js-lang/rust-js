// A `matches!` with a guard, negated, is a flag MIR's block gives, of a
// test one by the time it's read: the flag is the test's value, `const
// value = t;`, not `false` given first, then `true` where the test holds.

enum Shape {
    Cell(f64),
    Rows(Vec<Shape>),
}

fn check(m: &Shape) -> &'static str {
    if !matches!(m, Shape::Rows(rows) if matches!(rows.first(), Some(Shape::Rows(_)))) {
        return "not a matrix";
    }
    "a matrix"
}

fn deep(m: &Shape) -> bool {
    matches!(m, Shape::Rows(rows) if rows.len() > 1)
}

fn main() {
    let cell = Shape::Cell(1.0);
    let row = Shape::Rows(vec![Shape::Cell(1.0)]);
    let matrix = Shape::Rows(vec![Shape::Rows(vec![]), Shape::Cell(2.0)]);
    let empty = Shape::Rows(vec![]);
    for m in [&cell, &row, &matrix, &empty] {
        println!("{} {}", check(m), deep(m));
    }
    if let Shape::Cell(value) = cell {
        println!("{value}");
    }
}
