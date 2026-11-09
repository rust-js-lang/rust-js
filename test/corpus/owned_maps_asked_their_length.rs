// A map of an array's owned items with a destructor, made knowing its back,
// asked its length: by generic code that steps its back, and of a map of
// its `rev()`, which knows its length as its source does (ADR 0456; #382);
// and a generic one's map held, std's struct, asked its hint (#851).
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
// A map of a generic `ExactSizeIterator`, held to drop what its closure
// holds, std's compiled struct, asked its hint: its source's, through the
// map, not a `len` the struct has only where it's given one (#851).
fn hinted<I: DoubleEndedIterator<Item = Loud> + ExactSizeIterator>(src: I) -> (usize, Option<usize>) {
    let held = Loud(9);
    let it = src.map(move |x| {
        let _held = &held;
        x.0 + 1
    });
    it.size_hint()
}

fn main() {
    println!(
        "{:?}",
        ends(vec![Loud(1), Loud(2), Loud(3)].into_iter().map(|l| l.0 * 10))
    );
    let mapped = vec![Loud(4), Loud(5)].into_iter().map(|l| l);
    println!("{}", mapped.rev().map(|l| l.0).len());
    println!("{:?}", hinted(vec![Loud(11), Loud(12)].into_iter()));
    println!("end");
}
