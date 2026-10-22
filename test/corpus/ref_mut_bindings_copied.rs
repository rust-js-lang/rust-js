// A binding of a pattern matched against a `&mut`, `let (a, _) = &mut t`,
// writes the field it binds: a copy of the whole made before is its own,
// of a tuple, a struct, a nested tuple, through `if let`, `match` and a
// `for` over `iter_mut()`.
#[derive(Clone, Copy, Debug)]
struct Point {
    x: i32,
    y: i32,
}

fn main() {
    let mut pair = (1, 2);
    let kept = pair;
    let (first, _) = &mut pair;
    *first = 10;
    println!("{kept:?} {pair:?}");

    let mut point = Point { x: 1, y: 2 };
    let before = point;
    let Point { x, .. } = &mut point;
    *x = 7;
    println!("{before:?} {point:?}");

    let mut nested = ((1, 2), 3);
    let old = nested;
    let ((inner, _), _) = &mut nested;
    *inner = 5;
    println!("{old:?} {nested:?}");

    let mut maybe = Some(Point { x: 0, y: 0 });
    let saved = maybe;
    if let Some(Point { y, .. }) = &mut maybe {
        *y = 9;
    }
    match &mut point {
        Point { x, y } => *x += *y,
    }
    println!("{saved:?} {maybe:?} {point:?}");

    let mut points = vec![(1, 1), (2, 2)];
    let first_point = points[0];
    for (n, _) in points.iter_mut() {
        *n *= 10;
    }
    println!("{first_point:?} {points:?}");
}
