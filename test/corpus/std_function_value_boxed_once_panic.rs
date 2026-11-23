//@ run-fail: called `Option::unwrap()` on a `None` value
// `Option::unwrap` boxed and made a `Box<dyn FnOnce>`, whose one impl calls
// it through `FnOnce::call_once`: its panic is at `call_once`'s line, the
// shim's (#2335).
fn main() {
    let once: Box<dyn FnOnce(Option<i32>) -> i32> = Box::new(Option::unwrap);
    println!("{}", once(None));
}
