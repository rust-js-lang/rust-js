// `fold(None, f)` is JS's `reduce(f, undefined)`: `reduce` counts its
// arguments, so a start of `undefined` is given, not left out, as a
// trailing `undefined` elsewhere is, else an empty slice's `reduce` throws
// and another's starts from its first item (ADR 0330).
fn largest(xs: &[i32]) -> Option<i32> {
    xs.iter().copied().fold(None, |m, x| match m {
        Some(y) if y >= x => Some(y),
        _ => Some(x),
    })
}

fn joined(words: &[&str]) -> Option<String> {
    words.iter().fold(None, |acc: Option<String>, w| match acc {
        None => Some(w.to_string()),
        Some(s) => Some(format!("{s}-{w}")),
    })
}

fn main() {
    println!("{:?} {:?} {:?}", largest(&[3, 9, 2]), largest(&[]), largest(&[-1]));
    println!("{:?} {:?}", joined(&["a", "b", "c"]), joined(&[]));
}
