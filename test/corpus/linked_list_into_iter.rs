// A `LinkedList`'s `into_iter()` owns the array the list is (ADR 0316),
// so what it hasn't given is dropped where it's dropped, from its front or
// stepped from its back, as std's `IntoIter` drops it (ADR 0970, #377).

use std::collections::LinkedList;

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let ll: LinkedList<Loud> = (30..33).map(Loud).collect();
    let first = ll.into_iter().next().map(|l| l.0);
    println!("{first:?}");
    let ll: LinkedList<Loud> = (40..44).map(Loud).collect();
    let found = ll.into_iter().find(|l| l.0 == 41).map(|l| l.0);
    println!("{found:?}");
    let mut ll: LinkedList<Loud> = (50..53).map(Loud).collect();
    ll.push_front(Loud(49));
    let back = ll.into_iter().rev().next().map(|l| l.0);
    println!("{back:?}");
    let ll: LinkedList<u32> = (1..4).collect();
    let v: Vec<u32> = ll.into_iter().map(|x| x * 2).collect();
    println!("{v:?}");
}
