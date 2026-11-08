// An iterator a generic struct's field holds, stepped from its back: a
// tuple struct's `held.0`, read `held[0]`, is the iterator, not its array,
// `$rest(held[0])` reversed, where it was `held[0].toReversed()`, which
// threw; std's compiled `Map` in one instantiation's field beside another
// instantiation's range stepped from its back, given its compiled back
// where it's stepped, where its read was taken for one made knowing it and
// `it.back` threw; and a value given beside a `&mut` of such a field,
// `replaced(&mut held.0, next)`, which the callee may write there, made
// knowing its back too (#1013).
struct Held<T>(T);

struct Loud(u8);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn odd(x: &u32) -> bool {
    x % 2 == 1
}

fn replaced<I: Iterator<Item = u32>>(it: &mut I, next: I) -> I {
    std::mem::replace(it, next)
}

fn main() {
    let v = vec![1, 2, 3];
    let items = Held(v.iter());
    println!("{:?}", items.0.rev().collect::<Vec<_>>());
    let owned = Held(vec![4, 5, 6].into_iter());
    println!("{:?}", owned.0.rev().collect::<Vec<_>>());
    let mut range = Held((0..u32::MAX).rev());
    println!("{:?} {:?}", range.0.nth(4_000_000_000), range.0.next_back());
    let mapped = Held(vec![Loud(4), Loud(5)].into_iter().map(|l| l));
    println!("{:?}", mapped.0.rev().map(|l| l.0).collect::<Vec<_>>());
    let mut filtered = Held((0..6).filter(odd));
    println!("{:?}", filtered.0.next());
    let old = replaced(&mut filtered.0, (6..10).filter(odd));
    println!("{:?} {:?}", filtered.0.next_back(), old.collect::<Vec<_>>());
    println!("{:?}", filtered.0.collect::<Vec<_>>());
}
