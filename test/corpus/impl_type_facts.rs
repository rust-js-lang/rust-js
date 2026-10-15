// A trait impl's method that asks the size of its impl's type parameter,
// through a generic function, as arrayvec's `try_from` steps by bytes:
// its impl's dictionary is made of it, `wrapFootprint(TSize)`, one for
// each size, and gives it its method (ADR 0145).
use std::mem;

struct Wrap<T>(Vec<T>);

fn bytes<T>(n: usize) -> usize {
    mem::size_of::<T>() * n
}

trait Footprint {
    fn footprint(&self) -> usize;
}

impl<T> Footprint for Wrap<T> {
    fn footprint(&self) -> usize {
        bytes::<T>(self.0.len())
    }
}

fn total<F: Footprint>(f: &F) -> usize {
    f.footprint()
}

fn main() {
    let a = Wrap(vec![1u64, 2, 3]);
    let b = Wrap(vec![1u8]);
    println!("{} {} {}", a.footprint(), total(&b), total(&a));
}
