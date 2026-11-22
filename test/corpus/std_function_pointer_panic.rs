//@ run-fail: called `Option::unwrap()` on a `None` value
// `Option::unwrap` made a `fn` pointer: its shim gives it the location where
// it's defined, std's own line (#2335, ADR 0427).
fn main() {
    let f: fn(Option<i32>) -> i32 = Option::unwrap;
    println!("{}", f(Some(1)));
    println!("{}", f(None));
}
