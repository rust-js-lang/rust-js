// A map of an array's owned items with a destructor, made knowing its back,
// asked its length: by generic code that steps its back, and of a map of
// its `rev()`, which knows its length as its source does (ADR 0456; #382).
struct Loud(u32);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn ends<I: ExactSizeIterator + DoubleEndedIterator<Item = u32>>(mut it: I) -> (usize, Option<u32>, usize) {
    let a = it.len();
    let b = it.next_back();
    (a, b, it.len())
}
fn main() {
    println!(
        "{:?}",
        ends(vec![Loud(1), Loud(2), Loud(3)].into_iter().map(|l| l.0 * 10))
    );
    let mapped = vec![Loud(4), Loud(5)].into_iter().map(|l| l);
    println!("{}", mapped.rev().map(|l| l.0).len());
    println!("end");
}
