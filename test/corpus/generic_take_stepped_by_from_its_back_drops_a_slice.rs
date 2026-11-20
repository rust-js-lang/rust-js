// A `take(n)` in generic code of a `Vec`'s items with a destructor,
// stepped by a `step_by` from its back, through a `rev()` or a `step_by`'s
// own `next_back`, asks its source's `nth_back`, as std's `Take::nth_back`
// does, which drops what it passes over as one slice, front to back, not
// each as it's passed, back to front (#834, #2214).
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn items() -> Vec<D> {
    (1..=7).map(D).collect()
}

fn reversed<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(it: I) -> Vec<u32> {
    it.take(6).rev().step_by(3).map(|d| d.0).collect()
}

fn from_its_back<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(it: I) -> Vec<u32> {
    it.take(6).step_by(3).rev().map(|d| d.0).collect()
}

fn main() {
    println!("{:?}", reversed(items().into_iter()));
    println!("{:?}", from_its_back(items().into_iter()));
}
