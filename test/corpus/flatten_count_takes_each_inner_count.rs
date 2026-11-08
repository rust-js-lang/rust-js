// `count()` of a `flatten()` or a `flat_map` is std's `FlattenCompat::count`:
// each inner iterator's own `count`, the crate's where it writes one, of
// the one it's in first, then each its source makes, not `Iterator`'s
// default, which steps each by `next` (#736): made in place, given to
// generic code, held in a variable, fused, skipped and stepped part-way
// first, from both ends too, its back inner's own `count` last, stepped
// from its back then skipped, std's struct held so too (#1048); of inner
// iterators that hold what has a destructor, dropped as
// each is counted, the crate's iterator with a destructor and std's
// `count` and `last` too, whose drop is its value's (#766); and one whose
// inner iterator's own `count` panics drops what's left of its source.
use std::panic::catch_unwind;

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

/// Counts down from `.0`, with its own `count`, which panics at 13.
struct Own(u32);

impl Iterator for Own {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            return None;
        }
        self.0 -= 1;
        Some(self.0)
    }
    fn count(self) -> usize {
        println!("own count {}", self.0);
        if self.0 == 13 {
            panic!("count");
        }
        self.0 as usize
    }
}

impl DoubleEndedIterator for Own {
    fn next_back(&mut self) -> Option<u32> {
        self.next().map(|n| n + 100)
    }
}

/// Holds a `D` and counts down from it, with its own `count`.
struct Holding(D);

impl Iterator for Holding {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0.0 == 0 {
            return None;
        }
        self.0.0 -= 1;
        Some(self.0.0)
    }
    fn count(self) -> usize {
        println!("holding count {}", self.0.0);
        self.0.0 as usize
    }
}

/// Holds a `D` and counts down from `left`, with std's `count` and `last`.
struct Kept {
    _held: D,
    left: u32,
}

impl Iterator for Kept {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.left == 0 {
            return None;
        }
        self.left -= 1;
        Some(self.left)
    }
}

fn kept(held: u32, left: u32) -> Kept {
    Kept { _held: D(held), left }
}

fn counted<I: Iterator>(it: I) -> usize {
    it.count()
}

fn counted_from_both_ends<I: DoubleEndedIterator<Item = u32>>(mut it: I) -> usize {
    println!("next {:?} back {:?}", it.next(), it.next_back());
    it.count()
}

fn skipped_from_its_back<I: DoubleEndedIterator<Item = u32>>(mut it: I) -> usize {
    println!("back {:?}", it.next_back());
    it.skip(1).count()
}

fn main() {
    println!("{}", vec![2u32, 3].into_iter().flat_map(Own).count());
    println!("{}", vec![Own(2), Own(3)].into_iter().flatten().count());
    println!("{}", (1..4u32).flat_map(Own).count());
    println!("{}", [2u32, 3].iter().flat_map(|&n| Own(n)).count());
    println!("generic {}", counted(vec![2u32, 3].into_iter().flat_map(Own)));
    println!("generic {}", counted(vec![Own(1), Own(4)].into_iter().flatten()));
    let held = vec![5u32, 6].into_iter().flat_map(Own);
    println!("held {}", held.count());
    println!("fused {}", vec![2u32, 3].into_iter().flat_map(Own).fuse().count());
    println!("skipped {}", vec![2u32, 3].into_iter().flat_map(Own).skip(1).count());
    println!("skipped {}", vec![Own(2), Own(3)].into_iter().flatten().skip(1).count());
    let mut it = vec![2u32, 3, 4].into_iter().flat_map(Own);
    println!("next {:?}", it.next());
    println!("rest {}", it.count());
    // Stepped from both ends first, its back inner's own `count` last.
    let mut it = vec![2u32, 3, 4].into_iter().flat_map(Own);
    println!("next {:?} back {:?}", it.next(), it.next_back());
    println!("ends {}", it.count());
    println!("ends {}", counted_from_both_ends(vec![2u32, 3, 4].into_iter().flat_map(Own)));
    // Stepped from its back, then skipped: each inner's own `count` still,
    // as a `skip`'s asks its source's.
    println!("back skipped {}", skipped_from_its_back(vec![2u32, 3, 4].into_iter().flat_map(Own)));
    // Std's struct held, stepped from its back, then skipped: its own
    // `count`, which the `skip`'s asks (#1048).
    let mut it = vec![2u32, 3, 4].into_iter().flat_map(Own);
    println!("held back {:?}", it.next_back());
    println!("held back skipped {}", it.skip(1).count());
    let mut it = vec![Own(2), Own(3), Own(4)].into_iter().flatten();
    println!("held back {:?}", it.next_back());
    println!("held back skipped {}", it.skip(2).count());
    let mut it = vec![2u32, 3].into_iter().flat_map(Own).fuse();
    println!("fused back {:?}", it.next_back());
    println!("fused back skipped {}", it.skip(1).count());
    // Std's own `count` of each inner, `Map`'s of the crate's, which isn't
    // the crate's: each stepped by `next`.
    println!("mapped {}", vec![2u32, 3].into_iter().flat_map(|n| Own(n).map(|x| x + 1)).count());
    println!("vecs {}", vec![vec![D(1), D(2)], vec![D(3)]].into_iter().flatten().count());
    println!("holding {}", vec![4u32, 5].into_iter().map(|n| Holding(D(n))).flatten().count());
    println!("kept {}", vec![kept(40, 2), kept(41, 1)].into_iter().flatten().count());
    println!("kept last {:?}", vec![kept(42, 2), kept(43, 1)].into_iter().flatten().last());
    let r = catch_unwind(|| vec![D(20), D(13), D(21)].into_iter().flat_map(|d| Own(d.0)).count());
    println!("panicked {}", r.is_err());
    let r = catch_unwind(|| counted(vec![D(22), D(13), D(23)].into_iter().flat_map(|d| Own(d.0))));
    println!("panicked {}", r.is_err());
    let r = catch_unwind(|| vec![D(24), D(13), D(25)].into_iter().flat_map(|d| Own(d.0)).skip(1).count());
    println!("panicked {}", r.is_err());
    let r = catch_unwind(|| vec![2u32, 13, 4].into_iter().map(|n| (D(n + 30), Own(n))).flat_map(|(_, o)| o).count());
    println!("panicked {}", r.is_err());
}
