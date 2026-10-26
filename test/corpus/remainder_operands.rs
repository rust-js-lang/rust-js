// A signed remainder given to `+`, `-` or `*` has no wrap of its own,
// React's `baseLength - (baseLength % 5)`: its `-0`, `-10 % 5`, gives what `0`
// does, but a zero's sign, which the operator's wrap makes `0`. Each at
// the edges, a type's `MIN` and `MAX`, a narrower type's too.

fn rest(length: i32) -> i32 {
    length - (length % 5)
}

fn mixed(a: i32, b: i32, d: i32) -> (i32, i32, i32, i32) {
    (a % d + b, b - a % d, (a % d) * b, a % d * (b % d))
}

fn narrow(a: i8, d: i8) -> (i8, i8) {
    (a - a % d, (a % d) * a)
}

fn main() {
    for length in [0, 4, 5, 13, -5, -10, -13, i32::MIN, i32::MAX] {
        println!("{length}: {}", rest(length));
    }
    for (a, b, d) in [
        (-10, 0, 5),
        (-10, 3, 5),
        (i32::MIN, i32::MAX, 7),
        (i32::MAX, -1, 3),
        (-7, i32::MIN, 2),
    ] {
        println!("{:?}", mixed(a, b, d));
    }
    for (a, d) in [(-128, 3), (-120, 5), (127, 2), (-1, 1)] {
        println!("{:?}", narrow(a, d));
    }
}
