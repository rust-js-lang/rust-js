// `flatten` of `Option`s keeps the `Some`s, through a reference too, as
// `iter()` gives them: `Some(&x)` of `&Some(x)`. Of arrays, each one's items.
fn joined(parts: &[Option<&str>]) -> String {
    parts.iter().flatten().copied().collect::<Vec<_>>().join(" ")
}

fn main() {
    println!("{:?}", joined(&[Some("a"), None, Some("b"), None]));
    let numbers = vec![Some(1), None, Some(3)];
    println!("{}", numbers.iter().flatten().sum::<i32>());
    println!("{:?}", numbers.into_iter().flatten().collect::<Vec<_>>());
    let rows = vec![vec![1, 2], vec![], vec![3]];
    println!("{:?}", rows.iter().flatten().collect::<Vec<_>>());
    println!("{:?}", rows.into_iter().flatten().collect::<Vec<_>>());
}
