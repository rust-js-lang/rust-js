// `clone_from_slice` of a type parameter's items: each by its
// dictionary's `clone_from` where its impl writes one, as std's calls it,
// else a clone, arrayvec's `Clone::clone_from` of its prefix. A `Clone`
// impl's own `clone_from` is in its dictionary for it.
#[derive(Debug)]
struct Counted(u32);

impl Clone for Counted {
    fn clone(&self) -> Self {
        println!("clone {}", self.0);
        Counted(self.0)
    }

    fn clone_from(&mut self, source: &Self) {
        println!("clone_from {} <- {}", self.0, source.0);
        self.0 = source.0;
    }
}

#[derive(Clone, Debug)]
struct Plain(u32);

fn refill<T: Clone>(into: &mut [T], from: &[T]) {
    into.clone_from_slice(from);
}

trait Array {
    type Item;
}

impl<T, const N: usize> Array for [T; N] {
    type Item = T;
}

/// Of an associated type's items, smallvec's `A::Item`: the same.
fn refill_items<A: Array>(into: &mut [A::Item], from: &[A::Item])
where
    A::Item: Clone,
{
    into.clone_from_slice(from);
}

fn main() {
    let mut a = [Counted(1), Counted(2)];
    refill(&mut a, &[Counted(3), Counted(4)]);
    let mut b = [Plain(1)];
    refill(&mut b, &[Plain(5)]);
    let mut n = [1, 2];
    refill(&mut n, &[7, 8]);
    refill_items::<[Counted; 1]>(&mut a[..1], &[Counted(6)]);
    println!("{a:?} {} {n:?}", b[0].0);
}
