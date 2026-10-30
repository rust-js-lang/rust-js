//@ library-differs
// A generic impl's constant of its type parameter's size: its dictionary,
// made of the parameter's `TSize`, gives it (ADR 0145), where it was
// refused.
trait Size {
    const SIZE: usize;
}

struct W<T>(T);

impl<T> Size for W<T> {
    const SIZE: usize = std::mem::size_of::<T>();
}

fn size<S: Size>() -> usize {
    S::SIZE
}

fn main() {
    println!("{} {} {}", size::<W<u8>>(), size::<W<u64>>(), size::<W<(u8, u32)>>());
}
