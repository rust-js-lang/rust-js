// A clone of std's iterator over an array: where it is, to step on its own.
// Kept in a struct and cloned for each pass, as chrono's `DelayedFormat`
// does its items, and stepped through, then cloned (ADR 0181).

use std::borrow::Borrow;

struct Delayed<I> {
    items: I,
}

impl<I: Iterator<Item = B> + Clone, B: Borrow<u32>> Delayed<I> {
    fn total(&self) -> u32 {
        let mut sum = 0;
        for b in self.items.clone() {
            sum += *b.borrow();
        }
        sum
    }
}

fn main() {
    let x = 5u32;
    let one = Delayed { items: [&x].into_iter() };
    println!("{} {}", one.total(), one.total());
    let v = vec![1u32, 2, 3];
    let many = Delayed { items: v.iter() };
    println!("{} {}", many.total(), many.total());
    let owned = Delayed { items: [4u32, 6].into_iter() };
    println!("{} {}", owned.total(), owned.total());
    let moved = Delayed { items: vec![7u32, 8].into_iter() };
    println!("{} {}", moved.total(), moved.total());

    // Stepped through, then cloned: each goes on from where it was.
    let mut it = v.iter();
    it.next();
    let mut copy = it.clone();
    it.next();
    println!("{:?} {:?} {:?}", copy.next(), it.next(), copy.next());
    let mut a = [7, 8, 9].into_iter();
    a.next();
    let b = a.clone();
    println!("{:?} {:?}", b.collect::<Vec<_>>(), a.next());

    // Owned items a clone copies: changing one doesn't change the other.
    let rows = vec![vec![1], vec![2]].into_iter();
    let again = rows.clone();
    for mut row in rows {
        row.push(0);
        print!("{row:?} ");
    }
    println!("{:?}", again.collect::<Vec<_>>());
}
