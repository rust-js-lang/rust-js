// A `map(|x| x)` hides its source's own `count`, as std's `Map` counts by
// its `fold` (#1240): so it isn't left out where what it maps passes a
// `count` on, a `fuse()`'s, or the crate's own `count` would answer.
struct Upto(u32);

impl Iterator for Upto {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 < 5 {
            self.0 += 1;
            Some(self.0)
        } else {
            None
        }
    }
    fn count(self) -> usize {
        println!("own count from {}", self.0);
        100
    }
}

fn counted<I: Iterator>(it: I) -> usize {
    it.count()
}

fn main() {
    println!("{}", counted(Upto(0).fuse().map(|x| x)));
    println!("{}", Upto(1).fuse().map(|x| x).count());
    println!("{}", counted(Upto(2).fuse()));
    println!("{}", Upto(3).map(|x| x).count());
}
