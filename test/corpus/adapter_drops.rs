// A lazy adapter of an iterator that owns items with a destructor drops
// what it hasn't given when it's dropped, and each item it discards as it
// discards it, in generic code too, of what its caller gives; `eq` drops
// each pair it compares (ADR 0465). One that ends before what it steps,
// `take_while`, drops the rest as it ends, so each here is dropped at once
// (case B).

use std::iter::{Chain, Take, Zip};

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

impl PartialEq for Loud {
    fn eq(&self, other: &Loud) -> bool {
        println!("eq {} {}", self.0, other.0);
        self.0 % 10 == other.0 % 10
    }
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
        Some(Loud(self.left))
    }
}

fn zip<I: IntoIterator, J: IntoIterator>(i: I, j: J) -> Zip<I::IntoIter, J::IntoIter> {
    i.into_iter().zip(j)
}

fn chain<I: IntoIterator, J: IntoIterator<Item = I::Item>>(i: I, j: J) -> Chain<I::IntoIter, J::IntoIter> {
    i.into_iter().chain(j)
}

fn firsts<I: IntoIterator>(i: I, n: usize) -> Take<I::IntoIter> {
    i.into_iter().take(n)
}

fn equal<I: IntoIterator, J: IntoIterator>(a: I, b: J) -> bool
where
    I::Item: PartialEq<J::Item>,
{
    a.into_iter().eq(b)
}

fn louds(base: u32, n: u32) -> Vec<Loud> {
    (base..base + n).map(Loud).collect()
}

fn main() {
    let mut zipped = louds(10, 3).into_iter().zip(louds(20, 2));
    let pair = zipped.next();
    println!("zip {:?}", pair.as_ref().map(|(a, b)| (a.0, b.0)));
    drop(pair);
    println!("zip {:?}", zipped.next().map(|(a, b)| (a.0, b.0)));
    println!("zip {:?}", zipped.next().map(|(a, b)| (a.0, b.0)));
    drop(zipped);

    let mut chained = louds(30, 2).into_iter().chain(louds(40, 2));
    println!("chain {:?}", chained.next().map(|l| l.0));
    drop(chained);

    let mapped = louds(50, 2).into_iter().map(|l| l.0 * 2);
    drop(mapped);

    let mut kept = louds(60, 4).into_iter().filter(|l| l.0 % 2 == 1);
    println!("filter {:?}", kept.next().map(|l| l.0));
    drop(kept);

    let mut numbered = louds(70, 3).into_iter().enumerate();
    println!("enumerate {:?}", numbered.next().map(|(i, l)| (i, l.0)));
    drop(numbered);

    let mut skipping = louds(80, 4).into_iter().skip_while(|l| l.0 < 82);
    println!("skip_while {:?}", skipping.next().map(|l| l.0));
    drop(skipping);

    let mut taking = louds(90, 4).into_iter().take_while(|l| l.0 < 91);
    println!("take_while {:?}", taking.next().map(|l| l.0));
    let ended = taking.next().is_none();
    drop(taking);
    println!("take_while ended {ended}");

    let mut stepped = louds(100, 5).into_iter().step_by(2);
    println!("step_by {:?}", stepped.next().map(|l| l.0));
    println!("step_by {:?}", stepped.next().map(|l| l.0));
    drop(stepped);

    let total: u32 = louds(110, 3).into_iter().inspect(|l| println!("see {}", l.0)).map(|l| l.0).sum();
    println!("inspect {total}");

    let mut fused = louds(120, 2).into_iter().fuse();
    println!("fuse {:?}", fused.next().map(|l| l.0));
    drop(fused);

    for (i, l) in louds(130, 3).into_iter().enumerate() {
        if i == 1 {
            println!("break at {}", l.0);
            break;
        }
    }
    let mut z = zip(louds(140, 3), louds(150, 3));
    println!("generic zip {:?}", z.next().map(|(a, b)| (a.0, b.0)));
    drop(z);
    let mut c = chain(louds(160, 2), louds(170, 2));
    println!("generic chain {:?}", c.next().map(|l| l.0));
    drop(c);
    let mut counted = chain(Countdown { left: 3, _tag: Loud(999) }, louds(240, 2));
    println!("generic chain of its own {:?}", counted.next().map(|l| l.0));
    drop(counted);
    let mut own = Countdown { left: 3, _tag: Loud(998) }.chain(louds(250, 2));
    println!("chain of its own {:?}", own.next().map(|l| l.0));
    drop(own);
    let mut t = firsts(louds(180, 3), 2);
    println!("generic take {:?}", t.next().map(|l| l.0));
    drop(t);
    let mut plain = zip(vec![1, 2, 3], vec![4]);
    println!("plain zip {:?} {:?}", plain.next(), plain.next());

    println!("eq {}", louds(190, 3).into_iter().eq(louds(200, 3)));
    println!("eq {}", louds(190, 3).into_iter().eq(vec![Loud(210), Loud(5), Loud(212)]));
    println!("equal {}", equal(louds(220, 2), louds(230, 2)));
    println!("end");
}
