//@ run-fail: called `Option::unwrap()` on a `None` value
//@ located-otherwise: #2335 a `&dyn Fn` of `Option::unwrap`, borrowed as a promoted constant, names its caller's line, where native names `Fn::call`'s, until how it's called is decided
// `&Option::unwrap` made a `&dyn Fn` and called through it: natively at
// `Fn::call`'s line, the shim's.
fn through(f: &dyn Fn(Option<i32>) -> i32) -> i32 {
    f(None)
}

fn main() {
    println!("{}", through(&Option::unwrap));
}
