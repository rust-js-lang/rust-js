// A `filter_map` or a `map_while` of items with a destructor, held and
// dropped, closes its source to drop what it hasn't given, as Rust drops
// the iterator it holds (ADR 0518).
struct Loud(u8);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn main() {
    let v = vec![Loud(1), Loud(2), Loud(3)];
    let mut it = v.into_iter().filter_map(|l| (l.0 != 1).then_some(l));
    println!("{:?}", it.next().map(|l| l.0));
    drop(it);
    let w = vec![Loud(4), Loud(5), Loud(6)];
    let mut m = w.into_iter().map_while(|l| Some(l.0));
    println!("{:?}", m.next());
    drop(m);
    println!("end");
}
