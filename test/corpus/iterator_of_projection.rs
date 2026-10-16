// `for _ in self` of `&mut` an iterator of the crate's whose `Item` is
// another's associated type, `A::Item`, smallvec's `IntoIter` draining
// itself in its `Drop` (ADR 0364).
trait Array {
    type Item;
}

impl<T, const N: usize> Array for [T; N] {
    type Item = T;
}

struct Gen<A: Array> {
    items: Vec<A::Item>,
}

impl<A: Array> Iterator for Gen<A> {
    type Item = A::Item;
    fn next(&mut self) -> Option<A::Item> {
        self.items.pop()
    }
}

impl<A: Array> Drop for Gen<A> {
    fn drop(&mut self) {
        let mut n = 0;
        for _ in self {
            n += 1;
        }
        println!("drained {n}");
    }
}

fn main() {
    let g: Gen<[u32; 2]> = Gen { items: vec![1, 2, 3] };
    drop(g);
}
