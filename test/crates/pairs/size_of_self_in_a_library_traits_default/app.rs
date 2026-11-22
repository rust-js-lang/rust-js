//! The library's default reached through its generic function, through
//! the consumer's own generic function and a generic caller relaying it,
//! and called directly: each the library's impl's dictionary's answer
//! (#2290).

use dep::{Fixed, Sz, Wrap};

fn mine<S: Sz>(s: &S) -> usize {
    s.al() + dep::through(s)
}

fn outer<U>(w: &Wrap<U>) -> usize {
    mine(w)
}

pub fn main() {
    println!(
        "{} {} {}",
        dep::through(&Wrap([1u32, 2])),
        Wrap([1u8, 2]).al(),
        dep::through(&Fixed(1, 2))
    );
    println!(
        "{} {} {}",
        mine(&Wrap([1u16, 2])),
        mine(&Fixed(3, 4)),
        outer(&Wrap([1u64, 2]))
    );
}
