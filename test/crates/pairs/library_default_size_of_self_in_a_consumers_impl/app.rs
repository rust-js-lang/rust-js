//! The consumer's own impls of the library's trait, which copy its
//! default: the consumer compiles the default once over `Self`, and each
//! impl's entry gives it its type's layout, a constant of `C`, its
//! makers' of `Mine<T>`; reached directly, through the library's generic
//! function and through the consumer's own (#2291).

use dep::{Sz, Wrap};

struct C(u16, u8);

impl Sz for C {
    fn sz(&self) -> usize {
        0
    }
}

struct Mine<T>(T, u8);

impl<T> Sz for Mine<T> {
    fn sz(&self) -> usize {
        1
    }
}

fn mine<S: Sz>(s: &S) -> usize {
    s.al() + s.sz()
}

pub fn main() {
    println!("{} {} {}", C(5, 1).al(), dep::through(&C(1, 2)), mine(&C(3, 4)));
    println!(
        "{} {} {}",
        Mine(5u64, 1).al(),
        dep::through(&Mine(1u16, 2)),
        mine(&Mine((1u8, 2u32), 3))
    );
    println!("{} {}", dep::through(&Wrap([1u32, 2])), Wrap([1u8, 2]).al());
}
