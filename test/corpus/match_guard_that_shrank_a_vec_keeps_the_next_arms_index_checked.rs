//@ run-fail: index out of bounds: the len is 2 but the index is 2
// A guard that changes a `Vec`, `v.pop()`, then fails: the next arm's `v[i]`
// is tried after it, so the range analysis starts that arm from the state
// after the guard too, not only from before the match, and keeps its bounds
// check, which panics as native Rust's does (#1659).
fn pick(v: &mut Vec<i32>, i: usize, t: bool) -> i32 {
    if i < v.len() {
        match t {
            true if v.pop().is_none() => 0,
            _ => v[i],
        }
    } else {
        -1
    }
}

fn main() {
    let mut v = vec![1, 2, 3];
    println!("{}", pick(&mut v, 2, true));
}
