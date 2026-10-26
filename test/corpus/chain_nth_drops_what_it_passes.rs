// A chain's `nth` made in place drops what it passes over on either side,
// as std's `Chain::nth` does: `a` advanced by the item's drop where it has
// no `advance_by` of its own, then `b`'s `nth` given it too, the crate's
// iterator's or a compiled `fuse()`'s (ADR 0925).

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Count(u32, u32);

impl Iterator for Count {
    type Item = Loud;

    fn next(&mut self) -> Option<Loud> {
        self.0 += 1;
        (self.0 <= self.1).then(|| Loud(self.0))
    }
}

fn louds(from: u32, n: u32) -> Vec<Loud> {
    (from..from + n).map(Loud).collect()
}

fn main() {
    println!("{:?}", Count(100, 103).chain(Count(110, 112)).nth(4).map(|l| l.0));
    println!("--");
    println!("{:?}", Count(120, 123).chain(louds(130, 2)).nth(3).map(|l| l.0));
    println!("--");
    println!("{:?}", Count(0, 3).fuse().chain(louds(10, 3)).nth(4).map(|l| l.0));
    println!("--");
    println!(
        "{:?}",
        Count(60, 63).fuse().chain(Count(70, 72).fuse()).nth(4).map(|l| l.0)
    );
    println!("--");
    println!(
        "{:?}",
        Count(80, 83)
            .fuse()
            .chain(Count(90, 92).fuse())
            .skip(3)
            .next()
            .map(|l| l.0)
    );
    println!("--");
    println!(
        "{:?}",
        louds(20, 4).into_iter().skip(1).chain(louds(30, 2)).nth(3).map(|l| l.0)
    );
    println!("--");
    println!("{:?}", Count(40, 43).skip(1).chain(Count(50, 52)).nth(2).map(|l| l.0));
}
