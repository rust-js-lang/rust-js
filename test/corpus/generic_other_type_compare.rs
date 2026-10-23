// Generic code bounded by PartialEq<u32> or PartialOrd<u32>: its caller's
// impl, the dictionary of it, never a comparison of the type with itself.
#[derive(Debug, PartialEq)]
struct Cents(u32);
impl PartialEq<u32> for Cents { fn eq(&self, o: &u32) -> bool { self.0 == *o * 100 } }
impl PartialOrd<u32> for Cents {
    fn partial_cmp(&self, o: &u32) -> Option<std::cmp::Ordering> { self.0.partial_cmp(&(*o * 100)) }
}
fn has<T: PartialEq<u32>>(xs: &[T], n: u32) -> bool { xs.iter().any(|x| *x == n) }
fn under<T: PartialOrd<u32> + PartialEq<u32>>(xs: &[T], n: u32) -> Vec<bool> {
    xs.iter().map(|x| *x < n && *x != 0).collect()
}
fn main() {
    let cs = [Cents(300), Cents(150), Cents(0)];
    println!("{} {} {}", has(&cs, 3), has(&cs, 150), has(&[1u32, 2], 2));
    println!("{:?} {:?}", under(&cs, 2), under(&[0u32, 5, 9], 7));
    println!("{}", Cents(5) == Cents(5));
}
