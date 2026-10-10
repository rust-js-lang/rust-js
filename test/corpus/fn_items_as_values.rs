// A function as a value is the function (ADR 0125): the crate's by its
// name, a constructor's an arrow making what it makes, and one called
// through a `fn` pointer is called.
fn double(n: u32) -> u32 {
    n * 2
}

fn apply(f: fn(u32) -> u32, n: u32) -> u32 {
    f(n)
}

fn main() {
    let v: Vec<u32> = [1, 2, 3].iter().copied().map(double).collect();
    let o: Vec<Option<u32>> = v.iter().copied().map(Some).collect();
    println!("{:?} {:?} {}", v, o, apply(double, 5));
}
