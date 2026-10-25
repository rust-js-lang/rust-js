// A `Flatten` held is std's (ADR 0899): of `Vec`s, of arrays, whose
// `size_hint` counts what's left of each, of `Option`s, and of the crate's
// iterators, stepped from either end, folded both ways, its items dropped
// as Rust drops them; held in a struct and given back from generic code.
// One made in place stays the JS iterator it is.
use std::iter::Flatten;

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

#[derive(Clone)]
struct Countdown(u32);

impl Iterator for Countdown {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            None
        } else {
            self.0 -= 1;
            Some(self.0)
        }
    }
}

impl DoubleEndedIterator for Countdown {
    fn next_back(&mut self) -> Option<u32> {
        self.next().map(|n| n + 100)
    }
}

struct Rows<I>
where
    I: Iterator,
    I::Item: IntoIterator,
{
    cells: Flatten<I>,
}

fn rows<I>(it: I) -> Flatten<I>
where
    I: Iterator,
    I::Item: IntoIterator,
{
    it.flatten()
}

/// Of an iterator given as it is, `I: Clone`'s: its items' constant size
/// is its dictionary's.
fn rows_again<I>(it: &I) -> Flatten<I>
where
    I: Iterator + Clone,
    I::Item: IntoIterator,
{
    it.clone().flatten()
}

fn main() {
    // Of `Vec`s, stepped from both ends, what's left counted.
    let mut cells = vec![vec![1, 2], vec![], vec![3, 4, 5]].into_iter().flatten();
    println!("{:?} {:?}", cells.next(), cells.size_hint());
    println!("{:?} {:?}", cells.next_back(), cells.size_hint());
    println!("{:?}", cells.collect::<Vec<_>>());

    // Of arrays, whose constant size bounds what's left.
    let mut grid = [[1, 2, 3], [4, 5, 6]].into_iter().flatten();
    println!("{:?}", grid.size_hint());
    grid.next();
    grid.next_back();
    println!("{:?}", grid.size_hint());
    println!("{}", grid.rfold(0, |acc, x| acc * 10 + x));

    // Of references to arrays, the same.
    let pairs = [[1u8, 2], [3, 4]];
    let mut by_ref = pairs.iter().flatten();
    by_ref.next();
    println!("{:?} {}", by_ref.size_hint(), by_ref.fold(0, |a, &b| a + b));

    // Of `Option`s, the `Some`s.
    let mut some = vec![Some(1), None, Some(3)].into_iter().flatten();
    println!("{:?} {:?}", some.next_back(), some.size_hint());
    println!("{:?}", some.next());

    // Of the crate's iterators, from the back.
    let mut counted = vec![Countdown(2), Countdown(3)].into_iter().flatten();
    println!("{:?} {:?}", counted.next(), counted.next_back());
    println!("{:?}", counted.rev().collect::<Vec<_>>());

    // Its items dropped as it steps past them, the rest with it.
    let mut loud = vec![vec![Loud("a"), Loud("b")], vec![Loud("c")]].into_iter().flatten();
    let first = loud.next();
    println!("took {}", first.as_ref().map_or("none", |l| l.0));
    drop(first);
    drop(loud);

    // Held in a struct, made by generic code.
    let mut held = Rows {
        cells: rows(vec!["ab".chars(), "c".chars()].into_iter()),
    };
    println!("{:?} {:?}", held.cells.next(), held.cells.size_hint());
    println!("{}", held.cells.collect::<String>());
    let grids = [[1, 2], [3, 4]].into_iter();
    let mut again = rows_again(&grids);
    again.next();
    println!("{:?} {}", again.size_hint(), grids.count());

    // Made in place, stepped where it's made; asked its hint where it's
    // made, std's, which knows it.
    let total: i32 = vec![vec![1, 2], vec![3]].into_iter().flatten().sum();
    println!("{total} {:?}", [[1, 2], [3, 4]].iter().flatten().size_hint());
}
