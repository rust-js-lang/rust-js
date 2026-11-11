// A public re-export of std's items, by a glob, `pub use core::mem::*;`, as
// zerocopy's `core_reexport` writes, and by name: std's, in no JS module,
// exports nothing, as a type's or a trait's doesn't, and code naming them
// through the re-export calls std's own (rd802, #1315).
pub mod core_reexport {
    pub use core::*;

    pub mod mem {
        pub use core::mem::*;
    }
}

pub mod named {
    pub use core::mem::replace;
}

mod user {
    use super::core_reexport::mem;

    pub fn swapped(mut a: u32, mut b: u32) -> (u32, u32, usize) {
        mem::swap(&mut a, &mut b);
        (a, b, mem::size_of::<u64>())
    }
}

fn main() {
    println!("{:?}", user::swapped(1, 2));
    let mut v = vec![1, 2];
    let old = named::replace(&mut v, vec![3]);
    println!("{old:?} {v:?}");
    let taken = core_reexport::mem::take(&mut v);
    core_reexport::mem::forget(String::from("kept"));
    println!("{taken:?} {v:?} {}", core_reexport::cmp::max(3, 7));
}
