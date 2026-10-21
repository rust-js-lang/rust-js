// `iter::successors` of a value with a destructor (ADR 0465): each item it
// gives is the consumer's, dropped as the consumer drops it, and the next
// one it holds is dropped with it where it's stopped early, or never
// stepped, as Rust's `Successors` does.
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn tripled(l: &Loud) -> Option<Loud> {
    (l.0 < 20).then(|| Loud(l.0 * 3))
}

fn main() {
    let count = std::iter::successors(Some(Loud(1)), tripled).count();
    println!("count {count}");
    let first_big = std::iter::successors(Some(Loud(2)), tripled).find(|l| l.0 > 5).map(|l| l.0);
    println!("first {first_big:?}");
    let unused = std::iter::successors(Some(Loud(7)), tripled);
    drop(unused);
    println!("end");
}
