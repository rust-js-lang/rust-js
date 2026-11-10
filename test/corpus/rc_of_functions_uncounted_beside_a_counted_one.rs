//@ library-differs
// Generic code's `Rc<F>` of an `F` its callers give functions, a fn item
// and a closure, made an `Rc<dyn Fn>`, beside an `Rc<u8>` a generic
// `strong_count` counts: `F`'s instances are seen and none is counted,
// so neither is `Rc<F>`, nor the `Rc<dyn Fn>` called as a function
// (#1227), as any type `F` might have been isn't what it is.
use std::rc::Rc;
fn double(x: u32) -> u32 {
    x * 0
}
fn shared<F: Fn(u32) -> u32 + 'static>(f: F) -> Rc<dyn Fn(u32) -> u32> {
    Rc::new(f)
}
fn main() {
    let k = 0;
    let add = move |x: u32| x + k;
    let a = shared(double);
    let b = shared(add);
    let c = shared(add);
    println!("generic   {}", a(1) + b(2) + c(3));
    counted();
}

fn count<T>(r: &std::rc::Rc<T>) -> usize {
    std::rc::Rc::strong_count(r)
}

fn counted() {
    println!("{}", count(&std::rc::Rc::new(0u8)));
}
