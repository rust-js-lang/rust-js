// `cycle()` of an iterator other than std's over an array: a clone of it
// kept, and each time what it steps ends, a fresh clone of that stepped,
// as Rust's `Cycle` does, `$cycleOf(it, clone, next)`; of a range, the
// crate's own, and a type parameter's given as it is, by its dictionaries.
// Its items with a destructor are each its own; of an iterator with one,
// each it replaces is dropped after its clone is made, and both it holds
// as it's dropped, the one it keeps first. And `take(n)` of a type
// parameter given its drop, consumed, closed (ADR 0491).
struct Loud(u8);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

#[derive(Clone, Debug)]
struct Countdown(u32);

impl Iterator for Countdown {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            return None;
        }
        self.0 -= 1;
        Some(self.0)
    }
}

#[derive(Clone)]
struct Gen(u8);

impl Iterator for Gen {
    type Item = Loud;

    fn next(&mut self) -> Option<Loud> {
        self.0 += 1;
        if self.0 < 3 { Some(Loud(self.0)) } else { None }
    }
}

struct Counter {
    n: u8,
    tag: Loud,
}

impl Clone for Counter {
    fn clone(&self) -> Counter {
        println!("clone {}", self.tag.0);
        Counter { n: self.n, tag: Loud(self.tag.0 + 10) }
    }
}

impl Iterator for Counter {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        if self.n == 0 {
            return None;
        }
        self.n -= 1;
        Some(self.n)
    }
}

fn firsts<I: Iterator + Clone>(iter: I, n: usize) -> Vec<I::Item> {
    iter.cycle().take(n).collect()
}

fn two<I: Iterator>(iter: I) -> Vec<I::Item> {
    iter.take(2).collect()
}

fn main() {
    println!("{:?}", firsts(Countdown(2), 5));
    println!("{:?}", firsts(0..3, 7));
    println!("{:?}", firsts(vec!["a", "b"].into_iter(), 3));
    let r: Vec<u32> = (1..3).cycle().take(5).collect();
    println!("{r:?}");
    let e: Vec<u32> = (3..3).cycle().take(5).collect();
    println!("{e:?}");

    let tags: Vec<u8> = Gen(0).cycle().take(3).map(|l| l.0).collect();
    println!("{tags:?}");
    let held = firsts(Gen(0), 3);
    println!("held {}", held.len());
    drop(held);

    let counted: Vec<u8> = Counter { n: 2, tag: Loud(1) }.cycle().take(5).collect();
    println!("{counted:?}");
    println!("{:?}", firsts(Counter { n: 1, tag: Loud(2) }, 3));
    let unstepped = Counter { n: 3, tag: Loud(3) }.cycle();
    println!("made");
    drop(unstepped);
    println!("{:?}", two(Counter { n: 5, tag: Loud(4) }));
    println!("end");
}
