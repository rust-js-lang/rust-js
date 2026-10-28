// A map whose closure has a destructor, closed to drop it and held in a
// variable, given where generic code asks its length or its back: made
// knowing both where it's made, as Rust's `Map` passes on its source's,
// and its closure dropped once, when the map is; of a `fuse()` too, std's
// compiled struct, made knowing them by its own steps (#385).
struct Noisy(u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn ends<I>(mut it: I) -> (usize, Option<u32>, Option<u32>)
where
    I: DoubleEndedIterator<Item = u32> + ExactSizeIterator,
{
    (it.len(), it.next_back(), it.next())
}

fn lens<I: ExactSizeIterator<Item = u32>>(it: I) -> usize {
    it.len()
}

fn main() {
    let v = vec![1u32, 2, 3];
    let m = Noisy(5);
    println!(
        "{:?}",
        ends(v.iter().copied().map(move |x| {
            let k = &m;
            x + k.0
        }))
    );
    let n = Noisy(6);
    println!(
        "{}",
        lens(v.iter().map(move |x| {
            let k = &n;
            x + k.0
        }))
    );
    let o = Noisy(7);
    println!(
        "{:?}",
        ends(v.iter().copied().fuse().map(move |x| {
            let k = &o;
            x + k.0
        }))
    );
    println!("end");
}
