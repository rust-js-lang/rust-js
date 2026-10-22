// A value that holds a NaN isn't equal to itself, as Rust's derived
// `PartialEq` compares each field: `p == p`, a `Vec`'s, a tuple's, an
// array's and an enum's, and a clone that's the same JS object.
#[derive(PartialEq, Debug)]
struct Point {
    x: f64,
    y: f64,
}

#[derive(PartialEq)]
enum Reading {
    Value(f64),
}

fn main() {
    let p = Point { x: f64::NAN, y: 1.0 };
    let q = Point { x: 1.0, y: 2.0 };
    println!("{} {} {}", p == p, p != p, q == q);
    let v = vec![f64::NAN];
    let w = v.clone();
    println!("{} {}", v == v, v == w);
    let t = (1, f64::NAN);
    let a = [f64::NAN];
    let r = Reading::Value(f64::NAN);
    println!("{} {} {}", t == t, a == a, r == r);
}
