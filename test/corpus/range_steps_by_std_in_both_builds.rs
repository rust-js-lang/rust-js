// A range of each kind of bound stepped every way it's stepped, the same in
// the build of std's meaning, where `a..b` is std's struct, stepped by std's
// own compiled bodies and `Step`s, as in the plain build (ADR 1206).
use std::ops::Range;

fn first_two<I: Iterator<Item = u32>>(mut it: I) -> (Option<u32>, Option<u32>) {
    (it.next(), it.next())
}

fn main() {
    let mut r = 2..7u32;
    println!("{:?} {:?} {:?}", r.next(), r.nth(1), r.next_back());
    println!("{:?} {:?} {}", r, r.clone().last(), r.len());
    println!("{:?}", r.size_hint());
    let copy = r.clone();
    println!("{:?} {:?}", r.nth_back(0), copy);
    let d: Range<u32> = Default::default();
    println!("{:?} {:?} {}", d, d.clone().next(), d.is_empty());
    let mut total = 0;
    for i in 0..10u8 {
        total += i;
    }
    println!("{total}");
    println!("{}", (0..10).filter(|x| x % 3 == 0).count());
    println!("{:?}", (1..6i64).rev().step_by(2).collect::<Vec<_>>());
    println!("{:?}", (0..5).map(|x| x * x).zip(10..).collect::<Vec<_>>());
    let mut lent = 0..4u16;
    println!("{:?} {:?}", lent.by_ref().take(2).collect::<Vec<_>>(), lent);
    println!("{:?}", first_two(3..4));
    println!("{:?}", ('a'..'e').collect::<String>());
    println!("{:?} {:?}", ('x'..'z').nth(1), ('a'..'a').next());
    let big = u64::MAX - 2..u64::MAX;
    println!("{:?} {:?}", big.clone().count(), big.clone().last());
    let mut edge = 254u8..255;
    println!("{:?} {:?}", edge.next(), edge.next());
    println!("{:?} {:?}", (0..0u32).max(), (3..9u32).min());
    println!("{}", (0..100u32).nth(1000).is_none());
    println!("{:?}", (5..10i32).advance_by_shown());
    println!("{} {}", (5..2).len(), (2..5).len());
    let zipped = (0..5u32)
        .map(|x| {
            println!("read {x}");
            x
        })
        .zip(0..3u32)
        .count();
    println!("{zipped}");
    println!("{}", kept(Loud(1)..Loud(2)));
    let r = made(Loud(3), Loud(4));
    println!("{}", r.start.0 + r.end.0);
    println!("{}", kept(made(5u8, 9)));
}

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

// A range of what isn't a `Step`, generic code's too: no iterator, its
// fields dropped as any struct's are, in either build.
fn kept<T>(r: Range<T>) -> usize {
    let held = r;
    std::mem::size_of_val(&held.start).min(1)
}

fn made<T>(a: T, b: T) -> Range<T> {
    a..b
}

trait Shown {
    fn advance_by_shown(self) -> Vec<i32>;
}

impl Shown for Range<i32> {
    fn advance_by_shown(mut self) -> Vec<i32> {
        let skipped = self.nth(1);
        let mut out: Vec<i32> = skipped.into_iter().collect();
        out.extend(self.rev());
        out
    }
}
