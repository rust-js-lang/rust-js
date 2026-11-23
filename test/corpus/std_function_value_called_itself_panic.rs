//@ run-fail: called `Option::unwrap()` on a `None` value
// `Option::unwrap` in a local, given to `Iterator::map` and called itself:
// the call of it names its own line, as a call of `Option::unwrap` does,
// whatever shim `map`'s calls go through (#2335).
fn main() {
    let unwrap = Option::unwrap;
    let seen: Vec<i32> = vec![Some(1)].into_iter().map(unwrap).collect();
    println!("{seen:?}");
    println!("{}", unwrap(None::<i32>));
}
