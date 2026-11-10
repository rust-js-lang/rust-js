// An iterator `mem::replace` and `mem::take` give back is the one the place
// held, read as it was held, in a crate that reads a `Vec`'s capacity too,
// where std's own iterators are stepped as `$iter`s that know their hint
// (#1238).
use std::mem;

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let mut reversed = (0..3).rev();
    let old = mem::replace(&mut reversed, (5..7).rev());
    println!("{} {}", old.len(), reversed.len());
    println!("{:?} {:?}", old.collect::<Vec<_>>(), reversed.collect::<Vec<_>>());

    let mut items = vec![1, 2, 3].into_iter();
    items.next();
    let old = mem::replace(&mut items, vec![9].into_iter());
    println!("{} {:?}", old.len(), old.size_hint());
    println!("{:?} {:?}", old.collect::<Vec<_>>(), items.next());

    let items = [1, 2, 3];
    let mut slice = items.iter();
    slice.next();
    let old = mem::replace(&mut slice, items[..1].iter());
    println!("{} {}", old.len(), slice.len());
    println!("{:?}", old.rev().collect::<Vec<_>>());

    let mut words = "a,b,c".split(',');
    words.next();
    let old = mem::replace(&mut words, "x".split(','));
    println!("{:?} {:?}", old.collect::<Vec<_>>(), words.collect::<Vec<_>>());

    let mut loud = vec![Loud(4), Loud(5)].into_iter();
    let old = mem::replace(&mut loud, vec![Loud(6)].into_iter());
    drop(old);
    println!("old dropped");
    drop(loud);

    let mut grown = Vec::new();
    grown.extend((0..4).rev());
    println!("{}", grown.capacity());
}
