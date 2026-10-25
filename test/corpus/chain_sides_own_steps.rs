// A `chain` asks each side as std's `Chain` does: its `last` is each
// side's own `last`, `b`'s where it has one, `a`'s dropped then (#232);
// a `skip`'s `last` its source's after what it skips; and its `nth` each
// side's `advance_by`, which a `Vec`'s, reversed or not, does as one slice
// (#250).
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn louds(from: u32, n: u32) -> Vec<Loud> {
    (from..from + n).map(Loud).collect()
}

fn main() {
    println!("{:?}", louds(1, 2).into_iter().chain(louds(3, 2)).last().map(|l| l.0));
    println!("{:?}", louds(5, 2).into_iter().chain(Vec::new()).last().map(|l| l.0));
    println!(
        "{:?}",
        louds(7, 2)
            .into_iter()
            .chain(louds(9, 1))
            .chain(louds(10, 2))
            .last()
            .map(|l| l.0)
    );
    println!(
        "{:?}",
        louds(12, 3).into_iter().chain(louds(15, 3)).skip(2).last().map(|l| l.0)
    );
    println!(
        "{:?}",
        louds(18, 2).into_iter().rev().chain(louds(20, 3)).nth(2).map(|l| l.0)
    );
    for loud in louds(23, 2).into_iter().rev().chain(louds(25, 3)).skip(3) {
        println!("got {}", loud.0);
    }
    let nested = louds(28, 2).into_iter().rev().chain(louds(30, 2)).chain(louds(32, 2));
    println!("{:?}", nested.skip(3).map(|l| l.0).collect::<Vec<_>>());
    let stepped = louds(34, 4).into_iter().rev().chain(louds(38, 1)).step_by(3).nth(1);
    println!("{:?}", stepped.map(|l| l.0));
    let skipped = louds(39, 4).into_iter().skip(1).chain(louds(43, 2)).nth(2);
    println!("{:?}", skipped.map(|l| l.0));
    // `Chain::count`: each side's own, a `Take`'s advancing a reversed
    // `Vec`, which drops what it passes as one slice (#283).
    let counted = louds(45, 2).into_iter().rev().take(2).chain(louds(47, 2)).count();
    println!("{counted}");
    // A chain's `nth` past a `fuse()`'s items, which it drops (#285).
    let fused = louds(50, 3).into_iter().fuse().chain(louds(53, 1)).nth(2);
    println!("{:?}", fused.map(|l| l.0));
    println!("end");
}
