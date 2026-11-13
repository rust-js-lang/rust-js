// `{:#?}` and `{:?}` of lists of integers, each item's text its own,
// `String`'s, as `$pretty` and `join` make it: `i64::MIN` and `u128::MAX`,
// BigInts, and a list of lists; and of floats, which aren't given as they
// are, `-0.0` and `1.5` shown as Rust shows them.
fn main() {
    let small: Vec<i32> = vec![-1, 0, 7];
    let big: Vec<i64> = vec![i64::MIN, i64::MAX];
    let huge: Vec<u128> = vec![u128::MAX, 0];
    let nested: Vec<Vec<i32>> = vec![vec![1, 2], vec![], vec![-3]];
    let floats: Vec<f64> = vec![-0.0, 1.5, 2.0];
    println!("{:?} {:?} {:?}", small, big, huge);
    println!("{:?} {:?}", nested, floats);
    println!("{:#?}", small);
    println!("{:#?}", big);
    println!("{:#?}", huge);
    println!("{:#?}", nested);
    println!("{:#?}", floats);
    println!("{:#?}", (small, [1u8, 2]));
}
