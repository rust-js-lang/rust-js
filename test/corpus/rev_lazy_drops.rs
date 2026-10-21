// A `rev()` of a lazy chain over a `Vec` of items with a destructor (ADRs
// 0465, 0535): stepped from its back, each item it gives dropped where the
// program drops it, and those it never gives dropped as it's dropped, in
// the order Rust drops them, front to back.
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let mut same = vec![Loud(1), Loud(2), Loud(3)].into_iter().map(|l| l).rev();
    let last = same.next();
    println!("same {}", last.as_ref().map_or(0, |l| l.0));
    drop(last);
    drop(same);
    println!("--");

    let mut scaled = vec![Loud(4), Loud(5), Loud(6)]
        .into_iter()
        .map(|l| Loud(l.0 * 10))
        .rev();
    if let Some(l) = scaled.next() {
        println!("scaled {}", l.0);
    }
    println!("--");

    let kept: Vec<u32> = vec![Loud(7), Loud(8), Loud(9)]
        .into_iter()
        .rev()
        .map(|l| l.0)
        .collect();
    println!("{kept:?}");
}
