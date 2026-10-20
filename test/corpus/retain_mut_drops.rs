// `retain_mut` of a `Vec` or a `VecDeque` of values with a destructor drops
// each it doesn't keep as it goes, as `retain` does (ADR 0499).
use std::collections::VecDeque;
struct Loud(u8);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn main() {
    let mut v = vec![Loud(1), Loud(2), Loud(3), Loud(4)];
    v.retain_mut(|l| {
        l.0 += 10;
        l.0 % 2 == 0
    });
    println!("{:?}", v.iter().map(|l| l.0).collect::<Vec<_>>());
    let mut d: VecDeque<Loud> = vec![Loud(5), Loud(6)].into_iter().collect();
    d.retain_mut(|l| l.0 == 6);
    println!("{}", d.len());
    let mut n = vec![1, 2, 3];
    n.retain_mut(|x| {
        *x *= 3;
        *x > 3
    });
    println!("{n:?}");
}
