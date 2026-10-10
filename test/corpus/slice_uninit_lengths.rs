//@ run-fail: assertion `left == right` failed: destination and source slices have different lengths\n  left: 2\n right: 3
// `write_clone_of_slice` of a slice of another length panics with its `assert_eq!`.
use std::mem::MaybeUninit;
fn main() {
    let mut slots = [MaybeUninit::<i32>::uninit(); 2];
    slots.write_clone_of_slice(&[1, 2, 3]);
}
