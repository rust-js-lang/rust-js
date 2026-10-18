//@ run-fail: attempt to calculate the remainder with a divisor of zero
// `v[i] = a % b` checks its divisor where it's divided, before the index
// is made, so a zero divisor panics before the index's call runs; with
// none between, the division's own check is it. Found by a generated
// program (seed 12445863).
fn index(name: &str, i: usize) -> usize {
    println!("{name}: index");
    i
}

fn value(name: &str, x: u16) -> u16 {
    println!("{name}: value");
    x
}

fn main() {
    let mut v: Vec<u16> = vec![0, 0];
    v[index("first", 0)] = value("first", 7) % value("first", 4);
    let quotient = value("second", 9) / value("second", 2);
    println!("{v:?} {quotient}");
    v[index("third", 1)] = value("third", 7) % value("third", 0);
    println!("unreachable {v:?}");
}
