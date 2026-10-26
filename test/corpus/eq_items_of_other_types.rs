// `==` of a `Vec<String>` and a `Vec<&str>`, item by item, its JS arrow's
// names fresh: beside variables named as they'd be, and inside another,
// whose index the inner one's doesn't hide: TheAlgorithms/rust's
// n-queens test compares its solutions so with the ones it expects.
fn main() {
    let a: Vec<String> = vec!["p".to_string(), "q".to_string()];
    let x: Vec<&str> = vec!["p", "q"];
    let i: Vec<&str> = vec!["p", "z"];
    println!("{} {} {}", a == x, a == i, x == a);

    let rows: Vec<Vec<String>> = vec![
        vec!["a".to_string(), "d".to_string()],
        vec!["a".to_string(), "d".to_string()],
    ];
    let same: Vec<Vec<&str>> = vec![vec!["a", "d"], vec!["a", "d"]];
    let diagonal: Vec<Vec<&str>> = vec![vec!["a", "x"], vec!["y", "d"]];
    println!("{} {}", rows == same, rows == diagonal);
}
