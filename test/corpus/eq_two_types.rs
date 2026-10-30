//@ library-differs
// `==` of two types' items, `[A::Item] == [B::Item]`, smallvec's
// `PartialEq<SmallVec<B>>`: each by its bound, `A::Item: PartialEq<B::Item>`,
// its dictionary's (ADR 0053); each side evaluated once.
trait Array {
    type Item;
    fn items(&self) -> &[Self::Item];
}

impl<T, const N: usize> Array for [T; N] {
    type Item = T;
    fn items(&self) -> &[T] {
        self
    }
}

struct Small<A: Array>(A);

impl<A: Array, B: Array> PartialEq<Small<B>> for Small<A>
where
    A::Item: PartialEq<B::Item>,
{
    fn eq(&self, other: &Small<B>) -> bool {
        self.0.items()[..] == other.0.items()[..]
    }
}

fn main() {
    println!("{} {}", Small([1, 2]) == Small([1, 2]), Small([1, 2]) == Small([1, 3]));
}
