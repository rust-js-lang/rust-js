//@ library-differs
// A trait's default of the trait's const parameter, `N` of `Scaled<const N:
// usize>`, and of one of its own, `M`, copied into an impl of more
// parameters than its trait, `impl<T, U, const K: usize> Scaled<3> for S<T,
// U, K>`: in the impl's terms the trait's `N` is the impl's argument, `3`,
// `K` the impl's own, and `M` numbered after the impl's, where `M` had the
// number of the impl's `K` (ADR 0135, #803, #804).

pub trait Scaled<const N: usize> {
    fn base(&self) -> usize;

    fn scaled(&self) -> usize {
        self.base() * N
    }

    // And one of its own, numbered after the impl's in the impl's terms.
    fn times<const M: usize>(&self) -> usize {
        self.base() * M + N
    }
}

pub struct S<T, U, const K: usize>(T, U);

impl<T, U, const K: usize> Scaled<3> for S<T, U, K> {
    fn base(&self) -> usize {
        K
    }
}

fn main() {
    println!("{}", S::<u8, (), 4>(1, ()).scaled());
    println!("{}", S::<&str, u8, 5>("x", 2).scaled());
    println!("{}", S::<u8, (), 4>(1, ()).times::<7>());
}
