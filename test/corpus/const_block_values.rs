// A `const { .. }` block whose value is no value tree, a `String`'s or a
// `MaybeUninit`'s, is a constant of its module, as a named one is; and a
// static of one is too, as a `String`, a JS string, nothing changes in place.
use std::mem::MaybeUninit;

static GREETING: String = String::new();

fn main() {
    let mut s = const { String::new() };
    s.push('a');
    println!("{s} {} {}", const { String::new() }.len(), GREETING.is_empty());
    let mut slot = const { MaybeUninit::<(i32, i32)>::uninit() };
    slot.write((1, 2));
    println!("{:?}", unsafe { slot.assume_init() });
}
