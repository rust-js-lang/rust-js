// A `const { .. }` block whose value is no value tree, a `String`'s or a
// `MaybeUninit`'s, is a constant of its module, as a named one is.
use std::mem::MaybeUninit;

fn main() {
    let mut s = const { String::new() };
    s.push('a');
    println!("{s} {}", const { String::new() }.len());
    let mut slot = const { MaybeUninit::<(i32, i32)>::uninit() };
    slot.write((1, 2));
    println!("{:?}", unsafe { slot.assume_init() });
}
