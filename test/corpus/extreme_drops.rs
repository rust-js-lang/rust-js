// Picking the greatest or the least of items with a destructor drops each
// it passes over as it goes, as std's `max_by` and `min_by` fold them,
// `compare(kept, item)`, a key made once for each; and generic code that
// consumes an iterator its caller gives drops it after, as its caller's
// drop says (ADRs 0098, 0421): a crate's iterator's own `Drop` runs then.

use std::cmp::Ordering;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Loud(u32, &'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}{}", self.0, self.1);
    }
}

fn loud() -> Vec<Loud> {
    vec![Loud(2, "a"), Loud(5, "b"), Loud(1, "c"), Loud(5, "d"), Loud(1, "e")]
}

fn by_name(a: &Loud, b: &Loud) -> Ordering {
    println!("compare {}{} {}{}", a.0, a.1, b.0, b.1);
    a.1.cmp(b.1)
}

fn rank(l: &Loud) -> u32 {
    println!("rank {}{}", l.0, l.1);
    l.0
}

fn greatest<I: IntoIterator>(items: I) -> Option<I::Item>
where
    I::Item: Ord,
{
    items.into_iter().max()
}

fn final_one<I: IntoIterator>(items: I) -> Option<I::Item> {
    items.into_iter().last()
}

// Counts down, holding a tag it drops when it's dropped.
struct Countdown {
    left: u32,
    _tag: Loud,
}

impl Iterator for Countdown {
    type Item = Loud;

    fn next(&mut self) -> Option<Loud> {
        self.left = self.left.checked_sub(1)?;
        Some(Loud(self.left, "n"))
    }
}

fn main() {
    println!("{:?}", loud().into_iter().max());
    println!("{:?}", loud().into_iter().min());
    println!("{:?}", loud().into_iter().max_by(by_name));
    println!("{:?}", loud().into_iter().min_by(by_name));
    println!("{:?}", loud().into_iter().max_by_key(|l| l.0));
    println!("{:?}", loud().into_iter().min_by_key(rank));
    println!("{:?}", Vec::<Loud>::new().into_iter().max());
    println!("{:?}", greatest(loud()));
    println!("{:?}", greatest(vec![Some(3), None, Some(1)]));
    println!("{:?}", greatest(vec![None::<u32>]));
    println!("{:?}", final_one(loud()));
    println!("{:?}", final_one(Countdown { left: 3, _tag: Loud(0, "tag") }));
}
