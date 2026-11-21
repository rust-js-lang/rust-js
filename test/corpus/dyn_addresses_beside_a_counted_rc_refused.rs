//@ compile-fail: comparing the addresses of two `dyn std::ops::Fn(u32) -> u32`s in a crate that counts an `Rc` of one (#1343)
// `ptr::addr_eq` of two `Rc<dyn Fn>`s' values, in a crate that counts an
// `Rc` of a `dyn Fn`, a type parameter's function made one, `shared`: its
// `dyn` isn't its own pointer, `{ value, impl }`, but an `Rc`'s value, and
// has no size to compare an address by. Native prints `false 6` then
// `true`. Refused, naming its open issue, where it was refused as a
// `size_of` of a value without one size, alone or beside
// rc_fn_ptr_eq_of_param.rs (#1343).
use std::rc::Rc;

fn double(n: u32) -> u32 {
    n * 2
}

fn shared<F: Fn(u32) -> u32 + 'static>(f: F) -> Rc<dyn Fn(u32) -> u32> {
    Rc::new(f)
}

fn main() {
    let a = shared(double);
    let b = shared(double);
    println!("{} {}", Rc::ptr_eq(&a, &b), a(1) + b(2));
    let first: Rc<dyn Fn(u32) -> u32> = Rc::new(double);
    let again = Rc::clone(&first);
    println!("{}", std::ptr::addr_eq(&*first, &*again));
}
