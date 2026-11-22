//@ run-fail: called `Option::unwrap()` on a `None` value
// `Option::unwrap` given to a generic `F: Fn`, which calls it through
// `Fn::call`: its panic is at `Fn::call`'s line, the shim's (#2335).
fn twice<F: Fn(Option<i32>) -> i32>(f: F) -> i32 {
    f(Some(1)) + f(None)
}

fn main() {
    println!("{}", twice(Option::unwrap));
}
