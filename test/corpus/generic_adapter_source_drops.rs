// An adapter a generic function makes of a type parameter given as it is,
// `items.take(2)`, closed to drop what it holds: the value its caller gave,
// as the crate's own iterator's is, not the JS iterator stepping it (ADR
// 0514).
pub struct Counter(pub u32);
impl Iterator for Counter {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 < 3 {
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
fn taken<I: Iterator<Item = u32>>(items: I) -> std::iter::Take<I> {
    items.take(2)
}
fn main() {
    println!("{}", taken(Counter(0)).sum::<u32>());
    let mut p = taken(Counter(1));
    println!("{:?}", p.next());
    drop(p);
    println!("{}", Counter(0).take(1).sum::<u32>());
    println!("end");
}
