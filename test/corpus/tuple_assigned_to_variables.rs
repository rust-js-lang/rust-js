// A tuple assigned to the variables the program has, `(a, b) = e`: one
// destructuring assignment, `[a, b] = e`, as JS writes it, `e` made whole
// before either is set, as Euclid's subtractions are. Its parts read the
// variables they set, which still have their old values.
fn gcd(a: u32, b: u32) -> u32 {
    let (mut a, mut b) = (a, b);
    while a != 0 && b != 0 && a != b {
        (a, b) = if a > b { (a - b, b) } else { (a, b - a) };
    }
    if a != 0 {
        a
    } else {
        b
    }
}

fn fibonacci(n: u32) -> u64 {
    let (mut previous, mut current) = (0u64, 1u64);
    for _ in 0..n {
        (previous, current) = (current, previous + current);
    }
    previous
}

fn main() {
    println!("{} {} {}", gcd(1071, 462), gcd(0, 7), gcd(12, 12));
    println!("{:?}", (0..10).map(fibonacci).collect::<Vec<_>>());
}
