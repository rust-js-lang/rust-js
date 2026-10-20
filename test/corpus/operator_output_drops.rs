// A generic `a + b`'s `Output`, a std trait's associated type whose
// dictionary is given, `TAdd`, is dropped by that dictionary's
// `$dropOutput`, as a crate's trait's is (ADRs 0178, 0497).
use std::ops::Add;
struct Loud(u8);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
impl Add for Loud {
    type Output = Loud;
    fn add(self, o: Loud) -> Loud {
        Loud(self.0 + o.0)
    }
}
fn sum_drop<T: Add>(a: T, b: T) -> usize {
    let _x = a + b;
    0
}
fn main() {
    println!("{}", sum_drop(Loud(1), Loud(2)));
    println!("{}", sum_drop(1, 2));
}
