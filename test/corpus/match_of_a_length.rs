// A `match` of a slice's `len()`: an arm of a number knows the slice that
// long, so `list[0]` of `1 =>` is `list[0]`, and an arm that takes any
// after arms of 0 to n - 1 knows it at least n long, its `list[..len - 1]`
// a `slice` and its `list[len - 1]` an item, as React's
// ReactFlightClient joins a list, `switch (list.length) { case 0: ..`
// (rd21). An arm of a guard, or of a number not next, tells the arms after
// it nothing, so an index there is still checked, and panics as Rust's.
use std::panic;

fn join(list: &[&str], combinator: &str) -> String {
    match list.len() {
        0 => String::new(),
        1 => list[0].to_owned(),
        2 => list[0].to_owned() + " " + combinator + " " + list[1],
        _ => list[..list.len() - 1].join(", ") + ", " + combinator + " " + list[list.len() - 1],
    }
}

fn guarded(list: &[&str], skip: bool) -> String {
    match list.len() {
        0 => String::new(),
        1 if skip => "skipped".to_string(),
        _ => list[1].to_owned(),
    }
}

fn not_next(list: &[&str]) -> String {
    match list.len() {
        1 => list[0].to_owned(),
        _ => list[0].to_owned() + "!",
    }
}

fn main() {
    let lists: [&[&str]; 5] = [&[], &["a"], &["a", "b"], &["a", "b", "c"], &["a", "b", "c", "d"]];
    for list in lists {
        println!("{:?}", join(list, "and"));
    }
    println!("{:?}", guarded(&["a", "b"], false));
    println!("{:?}", guarded(&["a"], true));
    println!("{}", panic::catch_unwind(|| guarded(&["a"], false)).is_err());
    println!("{:?}", not_next(&["a"]));
    println!("{:?}", not_next(&["a", "b"]));
    println!("{}", panic::catch_unwind(|| not_next(&[])).is_err());
}
