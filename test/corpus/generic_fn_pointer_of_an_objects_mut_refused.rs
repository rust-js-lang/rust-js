//@ compile-fail: as a `fn` pointer whose `&mut T` is an object's or a slice's
// A generic function's instance whose `&mut T` is an object's, as a `fn`
// pointer: its value takes generic code's box, which a caller of the `fn`,
// giving the object, doesn't give (#1187).
fn view<T: ?Sized>(r: &mut T) -> &T {
    &*r
}

fn main() {
    let h: fn(&mut Vec<u8>) -> &Vec<u8> = view::<Vec<u8>>;
    let mut v = vec![1u8, 2];
    println!("{}", h(&mut v).len());
}
