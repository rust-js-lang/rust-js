//@ compile-fail: or whose raw pointer is a slice's
// A generic function's instance whose raw pointer is a slice's, as a `fn`
// pointer: its value takes generic code's handle, which a caller of the
// `fn`, giving the array, doesn't give, and generic code given the `fn`
// as an `F` does (#1187).
fn same<T: ?Sized>(p: *mut T) -> *mut T {
    p
}

fn main() {
    let s: fn(*mut [u8]) -> *mut [u8] = same::<[u8]>;
    let mut bytes = [1u8, 2, 3];
    let p = s(&mut bytes[..]);
    println!("{}", unsafe { (&(*p)).len() });
}
