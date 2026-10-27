// A float's `partial_cmp` compared with an `Ordering` is the comparison it
// is, `!(a < b)` of `a.partial_cmp(&b) != Some(Less)`, as React's
// findBoundingRects writes `!(otherBottom < targetTop)` (ADR 0966): `NaN`
// is neither less, more, nor equal, and `-0.0` equals `0.0`, in both. And
// `contains` of what `===` compares but `NaN` is `indexOf(x) >= 0`, which
// finds no `NaN`, as `==` doesn't, React's `nextInstanceRoots.indexOf(target)
// < 0`.
use std::cmp::Ordering;

fn tests(a: f64, b: f64) -> [bool; 6] {
    [
        a.partial_cmp(&b) != Some(Ordering::Less),
        a.partial_cmp(&b) == Some(Ordering::Less),
        a.partial_cmp(&b) != Some(Ordering::Greater),
        a.partial_cmp(&b) == Some(Ordering::Greater),
        a.partial_cmp(&b) == Some(Ordering::Equal),
        a.partial_cmp(&b) != Some(Ordering::Equal),
    ]
}

fn main() {
    let values = [1.0, 2.0, -0.0, 0.0, f64::NAN, f64::INFINITY];
    for &a in &values {
        for &b in &values {
            println!("{a} {b} {:?}", tests(a, b));
        }
    }
    let within = |top: f64, bottom: f64| bottom.partial_cmp(&top) != Some(Ordering::Less);
    println!("{} {}", within(1.0, 2.0), within(2.0, 1.0));
    for x in [f64::NAN, -0.0, 3.0] {
        if !values.contains(&x) {
            println!("{x} missing");
        } else {
            println!("{x} found");
        }
    }
}
