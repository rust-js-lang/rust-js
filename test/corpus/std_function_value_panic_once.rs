//@ run-fail: called `Option::unwrap()` on a `None` value
// `Option::unwrap` given to `Option::map`, whose `F: FnOnce` calls it so: its
// panic is at `FnOnce::call_once`'s line, the shim rustc calls it through (#2335).
fn main() {
    let nested: Option<Option<i32>> = Some(None);
    println!("{:?}", nested.map(Option::unwrap));
}
