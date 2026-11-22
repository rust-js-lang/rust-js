//@ run-fail: no value
// The crate's own `#[track_caller]` function given to `Iterator::map` panics
// where `FnMut::call_mut` calls it, as std's does, not where it's defined
// (#2335).
#[track_caller]
fn first(v: Option<i32>) -> i32 {
    match v {
        Some(n) => n,
        None => panic!("no value"),
    }
}

fn main() {
    let seen: Vec<i32> = vec![Some(1), None].into_iter().map(first).collect();
    println!("{seen:?}");
}
