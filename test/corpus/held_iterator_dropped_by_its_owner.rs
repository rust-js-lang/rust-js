// An iterator its owner drops, a variable's or a temporary's, drops what it
// holds where Rust drops it, at its scope's or its statement's end, not as
// it's done or an adapter in it ends (#206): a held `take_while`, `take`
// and `map_while` stepped to their end; a temporary's `take_while` ended
// inside `nth`, before the statement prints; a restarting source stepped
// past its `None`. One consumed where it's made, or a held one moved into
// a consumer on one path, drops it at its consumer's end, and one JS's
// own consumer takes as it's done or an adapter in it ends.
use std::collections::HashSet;

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Restart {
    n: u32,
}

impl Iterator for Restart {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        self.n += 1;
        if self.n == 2 || self.n >= 5 { None } else { Some(self.n) }
    }
}

impl Drop for Restart {
    fn drop(&mut self) {
        println!("drop restart {}", self.n);
    }
}

fn ds(from: u32) -> Vec<D> {
    vec![D(from), D(from + 5), D(from + 1)]
}

fn main() {
    {
        let mut it = ds(0).into_iter().take_while(|d| d.0 < 3);
        while let Some(d) = it.next() {
            println!("got {}", d.0);
        }
        println!("after take_while");
    }
    {
        let mut it = ds(10).into_iter().take(1);
        while let Some(d) = it.next() {
            println!("got {}", d.0);
        }
        println!("after take");
    }
    {
        let mut it = ds(20).into_iter().map_while(|d| (d.0 < 21).then_some(d.0));
        while let Some(x) = it.next() {
            println!("got {x}");
        }
        println!("after map_while");
    }
    println!(
        "{:?}",
        ds(30).into_iter().rev().take_while(|d| d.0 < 31).nth(0).map(|d| d.0)
    );
    let mut again = Restart { n: 0 }.map(|x| x * 10);
    let (a, b, c) = (again.next(), again.next(), again.next());
    println!("{a:?} {b:?} {c:?} after restart");
    let total: u32 = ds(40).into_iter().take(1).map(|d| d.0).sum();
    println!("{total}");
    let mut some = ds(50).into_iter().take(2);
    println!("{:?}", some.next().map(|d| d.0));
    if total > 0 {
        let rest: Vec<u32> = some.map(|d| d.0).collect();
        println!("rest {rest:?}");
    }
    // Collected by JS's own `new Set(..)`, which closes nothing at its end:
    // each source closed as it's done, or as an adapter over it ends.
    let firsts: HashSet<u32> = ds(60).into_iter().take(1).map(|d| d.0).collect();
    let backs: HashSet<u32> = ds(70).into_iter().rev().map(|d| d.0).collect();
    let kept: HashSet<u32> = ds(100).into_iter().take(2).filter(|d| d.0 > 0).map(|d| d.0).collect();
    let scanned: HashSet<u32> = ds(110).into_iter().scan(0, |_, d| (d.0 < 111).then_some(d.0)).collect();
    let sizes = [firsts.len(), backs.len(), kept.len(), scanned.len()];
    println!("{sizes:?} end");
}
