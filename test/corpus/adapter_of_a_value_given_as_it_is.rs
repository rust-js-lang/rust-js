// An adapter made where generic code holds a type parameter given as it
// is, `I: Clone`'s, closed early: what it drops is the value its caller
// gave, the crate's own iterator, as Rust drops it with the adapter, not
// the JS iterator stepping it (ADR 0514; #382).
#[derive(Clone)]
pub struct Counter(pub u32);
impl Iterator for Counter {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 < 5 {
            self.0 += 1;
            Some(self.0)
        } else {
            None
        }
    }
}
impl Drop for Counter {
    fn drop(&mut self) {
        println!("drop counter {}", self.0);
    }
}
fn first_two<I: Iterator<Item = u32> + Clone>(items: I) -> Vec<u32> {
    let v = items.take(2).collect();
    println!("collected");
    v
}
fn found<I: Iterator<Item = u32> + Clone>(items: I) -> Option<u32> {
    let r = items.map(|x| x * 3).find(|x| x % 2 == 0);
    println!("found");
    r
}
fn main() {
    println!("{:?}", first_two(Counter(0)));
    println!("{:?}", found(Counter(1)));
    println!("{:?}", first_two(Counter(2)));
    println!("end");
}
