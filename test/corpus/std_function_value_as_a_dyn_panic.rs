//@ run-fail: called `Option::unwrap()` on a `None` value
// `&Option::unwrap` made a `&dyn Fn` and called through it: natively at
// `Fn::call`'s line, the shim's.
fn through(f: &dyn Fn(Option<i32>) -> i32) -> i32 {
    f(None)
}

fn main() {
    println!("{}", through(&Option::unwrap));
}
