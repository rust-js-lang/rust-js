//@ compile-fail: a raw pointer to this place
// A pointer to what a `&mut` refers to, returned, a closure's captured
// variable: the caller's writes through it would reach a copy, not `n`, so
// it's refused, not taken as only read, a cell of the value (#1109).
fn main() {
    let mut n = 1u32;
    let mut get = || -> *mut u32 {
        let r = &mut n;
        r
    };
    let p = get();
    unsafe { *p += 41 };
    println!("{n}");
}
