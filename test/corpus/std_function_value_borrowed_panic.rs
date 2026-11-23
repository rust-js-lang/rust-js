//@ run-fail: called `Option::unwrap()` on a `None` value
// A reference to `Option::unwrap` given as an `F: FnMut`: `&F`'s impls call
// it through `Fn::call`, whatever `F`'s bound, so its panic is at
// `Fn::call`'s line, not `call_mut`'s (#2355).
fn apply<F: FnMut(Option<i32>) -> i32>(mut f: F) -> i32 {
    f(None)
}

fn main() {
    let unwrap = Option::unwrap;
    let by_ref = &unwrap;
    println!("{}", apply(by_ref));
}
