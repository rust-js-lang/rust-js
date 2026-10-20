// A collection made of another it's given, `VecDeque::from(v)`,
// `Vec::from([..])`, `LinkedList::from([..])`, keeps each item with a
// destructor, which it drops as it's dropped (ADR 0500).
use std::collections::{LinkedList, VecDeque};
struct Loud(u8);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn main() {
    let mut d = VecDeque::from(vec![Loud(1), Loud(2)]);
    d.pop_front();
    println!("{}", d.len());
    let v = Vec::from([Loud(3), Loud(4)]);
    println!("{}", v.len());
    let l = LinkedList::from([Loud(5)]);
    println!("{}", l.len());
    let s: Vec<u8> = [1u8, 2].to_vec();
    println!("{s:?}");
}
