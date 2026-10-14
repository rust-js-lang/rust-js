// A slice of floats' `contains` doesn't find `NaN`, as Rust's `==` doesn't:
// not `includes`, whose SameValueZero would (ADR 0036; the second
// architecture review: one of five predicates let floats through).

fn main() {
    let values = [f64::NAN, 1.0, -0.0];
    println!("{} {} {}", values.contains(&f64::NAN), values.contains(&1.0), values.contains(&0.0));
    let held: Vec<f64> = vec![f64::NAN];
    println!("{}", held.contains(&f64::NAN));
}
