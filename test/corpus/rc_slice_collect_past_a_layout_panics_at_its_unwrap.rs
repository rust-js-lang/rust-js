//@ run-fail: called `Result::unwrap()` on an `Err` value: LayoutError
// A collect into an `Rc<[T]>` past what a `Layout` of the slice holds
// panics where std's `Rc::<[T]>::allocate_for_slice` unwraps its
// `LayoutError`, `rc.rs`'s line, as std's hook says (#2489).
use std::rc::Rc;

fn main() {
    let small: Rc<[u8]> = (0..5u8).map(|i| i * 2).collect();
    println!("{small:?}");
    let huge: Rc<[u64]> = (0..usize::MAX).map(|i| i as u64).collect();
    println!("{}", huge.len());
}
