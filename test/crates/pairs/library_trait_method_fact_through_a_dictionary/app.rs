//! A call of the library's trait's generic method through a type
//! parameter's dictionary, of a concrete type and of the caller's own type
//! parameter, which its caller gives, a direct one, which compiles the library's impl
//! here (ADR 0914), and impls of this crate's of the trait whose methods ask
//! facts the contract gives, of the method's own type parameter, and facts
//! they answer themselves, of their own type parameter and of a type they
//! fix, each given to the library's generic code: each gives what native
//! Rust's does (#2285).
use dep::{SliceRandom, sized_of};

fn through<S: SliceRandom + ?Sized>(s: &S) -> usize {
    s.sized(7u32)
}

fn relayed<Y: Clone, S: SliceRandom + ?Sized>(s: &S, y: Y) -> usize {
    s.sized(y)
}

struct Wrap<T>([T; 2]);

impl<T> SliceRandom for Wrap<T> {
    type Item = T;
    fn choose(&self, at: usize) -> Option<&T> {
        self.0.get(at)
    }
    fn sized<X: Clone>(&self, x: X) -> usize {
        let _kept = x.clone();
        std::mem::size_of::<X>() * 100 + std::mem::size_of::<T>() * 10
    }
}

struct Fixed([u16; 3]);

impl SliceRandom for Fixed {
    type Item = u16;
    fn choose(&self, at: usize) -> Option<&u16> {
        self.0.get(at)
    }
    fn sized<X: Clone>(&self, x: X) -> usize {
        let _kept = x.clone();
        std::mem::size_of::<X>() * 100 + std::mem::size_of::<Self>()
    }
}

pub fn main() {
    let a = [1u8, 2, 3];
    println!("{} {}", through(&a[..]), relayed(&a[..], (1u8, 2u16)));
    println!("{:?} {}", a.choose(1), a.sized(5u64));
    println!("{}", sized_of(&a[..]));
    let w = Wrap([1u32, 2]);
    println!("{} {:?} {}", through(&w), w.choose(0), sized_of(&w));
    let f = Fixed([4, 5, 6]);
    println!("{} {:?} {}", through(&f), f.choose(2), sized_of(&f));
}
