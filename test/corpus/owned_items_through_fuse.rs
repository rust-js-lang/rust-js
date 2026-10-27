// Items with a destructor through a `fuse()`, an `enumerate()`, a `rev()`,
// `nth` and a generic adapter: each dropped where Rust drops it, what's
// passed over and what's left (#225, #286; #382).

struct Loud(u32);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn louds(n: u32) -> Vec<Loud> {
    (0..n).map(Loud).collect()
}
fn firsts<I: Iterator<Item = Loud>>(it: I) -> Option<u32> {
    it.map(|l| l.0).next()
}
fn backs<I: DoubleEndedIterator<Item = Loud> + ExactSizeIterator>(it: I) -> Option<(usize, u32)> {
    it.enumerate().rev().map(|(i, l)| (i, l.0)).next()
}
fn main() {
    let mut f = louds(4).into_iter().fuse();
    println!("{:?}", f.next().map(|l| l.0));
    println!("{:?}", f.nth(1).map(|l| l.0));
    drop(f);
    let r = louds(3).into_iter().fuse().rev().map(|l| l.0).next();
    println!("{r:?}");
    let c = louds(3).into_iter().fuse().map(|l| l.0).next();
    println!("{c:?}");
    println!("{:?}", firsts(louds(3).into_iter()));
    println!("{:?}", backs(louds(3).into_iter()));
    let mut held = louds(5).into_iter().rev().fuse();
    println!("{:?}", held.nth(2).map(|l| l.0));
    println!("{:?}", held.next_back().map(|l| l.0));
}
