//@ compile-fail: which `call_then_give` calls and gives back or takes in another parameter
// A `&mut` to a boxed function given for an `F` that's called and given
// back is refused (#1909): generic code would call it, so it must be the
// function, and its caller writes through what comes back, so it must be
// the place.

fn call_then_give<F: FnMut()>(mut f: F) -> F {
    f();
    f
}

fn main() {
    let mut callback: Box<dyn Fn()> = Box::new(|| println!("first"));
    let again = call_then_give(&mut callback);
    *again = Box::new(|| println!("replaced"));
    callback();
}
