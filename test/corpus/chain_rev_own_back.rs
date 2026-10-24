// A chain of the crate's own iterator that steps from its back, `rev()`'d
// where it's made or returned: its own `next_back`, as Rust's `Rev`
// calls it, never its items from the front reversed.
struct Down {
    n: u32,
}
impl Iterator for Down {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        (self.n > 0).then(|| {
            self.n -= 1;
            self.n
        })
    }
}
impl DoubleEndedIterator for Down {
    fn next_back(&mut self) -> Option<u32> {
        (self.n > 0).then(|| {
            self.n -= 1;
            100 + self.n
        })
    }
}
fn ends() -> impl DoubleEndedIterator<Item = u32> {
    Down { n: 3 }.chain(vec![7, 8])
}
fn mapped() -> impl DoubleEndedIterator<Item = u32> {
    Down { n: 3 }.map(|x| x * 2)
}
fn filtered() -> impl DoubleEndedIterator<Item = u32> {
    Down { n: 4 }.filter(|x| x % 2 == 0)
}
fn main() {
    println!("{:?}", Down { n: 3 }.chain(vec![7, 8]).rev().collect::<Vec<_>>());
    println!("{:?}", ends().rev().collect::<Vec<_>>());
    println!("{:?}", mapped().rev().collect::<Vec<_>>());
    println!("{:?}", filtered().next_back());
}
