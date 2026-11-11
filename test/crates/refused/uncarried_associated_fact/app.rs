//! A consumer's generic code that asks the size of a library trait's
//! associated type, which the library's dictionaries don't carry, as one of
//! its impls can't give it: refused, never read as `undefined` (#476).
use dep::{Boxed, Holder};

fn held_size<H: Holder>(holder: H) -> usize {
    let _ = holder.held();
    size_of::<H::Held>()
}

pub fn main() {
    println!("{}", held_size(Boxed(7u64)));
}
