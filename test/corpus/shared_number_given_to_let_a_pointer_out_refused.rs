//@ compile-fail: given to generic code that may let it out as a raw pointer
// A shared reference to a number, which JS holds as the number, given to
// generic code that may let it out as a raw pointer: its pointer would be
// of a copy (ADR 1114).
fn ptr_from_ref<T: ?Sized>(r: &T) -> *const T {
    r
}

fn main() {
    let n = 5u32;
    let p = ptr_from_ref(&n);
    println!("{}", unsafe { *p });
}
