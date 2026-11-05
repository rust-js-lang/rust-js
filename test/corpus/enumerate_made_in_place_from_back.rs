// An `enumerate()` of an adapter made in place, its JS chain, not std's
// struct, a `skip()`'s, a `take()`'s, a `rev()`'s or a `map()`'s,
// reversed: its map form, made knowing its back by its source where it's
// stepped. And of a `Vec`'s items themselves, of items with a destructor,
// one held and stepped by its own `next_back()` beside one reversed, each
// item dropped as Rust drops it (#712).
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let skipped: Vec<(usize, u32)> = vec![1, 2, 3, 4].into_iter().skip(1).enumerate().rev().collect();
    let taken: Vec<(usize, u32)> = vec![1, 2, 3, 4].into_iter().take(3).enumerate().rev().collect();
    let reversed: Vec<(usize, u32)> = (0..5u32).rev().enumerate().rev().collect();
    let mapped: Vec<(usize, u32)> = vec![Loud(1), Loud(2)].into_iter().map(|l| l.0).enumerate().rev().collect();
    println!("{skipped:?} {taken:?} {reversed:?} {mapped:?}");

    let mut it = vec![Loud(1), Loud(2), Loud(3)].into_iter().enumerate();
    let last = it.next_back().map(|(i, l)| (i, l.0));
    println!("last {last:?}");
    let r: Vec<_> = vec![Loud(7), Loud(8)]
        .into_iter()
        .enumerate()
        .rev()
        .map(|(i, l)| (i, l.0))
        .collect();
    println!("{r:?}");
}
