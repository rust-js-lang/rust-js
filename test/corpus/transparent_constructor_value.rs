// A `#[repr(transparent)]` struct's constructor passed as a function,
// `.map(Meters)`, makes what a call of it makes: its field (ADR 0584), of a
// number and of a `NonNull`, as smallvec's `ConstNonNull::new` passes it.
use std::ptr::NonNull;

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
struct Meters(f64);

#[repr(transparent)]
struct ConstNonNull<T>(NonNull<T>);

impl<T> ConstNonNull<T> {
    fn new(ptr: *const T) -> Option<Self> {
        NonNull::new(ptr as *mut T).map(Self)
    }

    fn as_ptr(self) -> *const T {
        self.0.as_ptr()
    }
}

fn main() {
    let lengths: Vec<Meters> = [1.5, 2.0].into_iter().map(Meters).collect();
    let first = Some(3.0).map(Meters);
    println!("{lengths:?} {first:?}");
    let items = [1, 2, 3];
    let pointer = ConstNonNull::new(items.as_ptr()).unwrap();
    let all = unsafe { std::slice::from_raw_parts(pointer.as_ptr(), 3) };
    println!("{all:?}");
}
