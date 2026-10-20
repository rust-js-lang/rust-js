// More of std's calls of values with a destructor (ADR 0509): an array's
// `map`, whose function owns each item; `unwrap_or_default`, of a `Result`
// its `Err` dropped once the default's made, as std drops it when it
// returns; `pop_if` and `pop_front_if`, which give back what they take; an
// `Option`'s `as_mut`; and what changes a vector's capacity, which drops
// nothing.
use std::collections::VecDeque;
#[derive(Debug)]
struct Loud(u8);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
impl Default for Loud {
    fn default() -> Self {
        println!("default");
        Loud(0)
    }
}
fn mapped() {
    let a = [Loud(1), Loud(2)];
    let b = a.map(|l| l.0 + 10);
    println!("{b:?}");
    let c = [Loud(3), Loud(4)].map(|l| Loud(l.0 + 10));
    println!("{:?}", c.iter().map(|l| l.0).collect::<Vec<_>>());
}
fn defaults() {
    let o: Option<Loud> = None;
    println!("option {}", o.unwrap_or_default().0);
    let r: Result<Loud, Loud> = Err(Loud(5));
    println!("err {}", r.unwrap_or_default().0);
    let k: Result<Loud, Loud> = Ok(Loud(6));
    println!("ok {}", k.unwrap_or_default().0);
    let n: Result<u8, Loud> = Err(Loud(7));
    println!("is ok {}", n.is_ok_and(|x| x > 0));
}
fn popped() {
    let mut v = vec![Loud(7), Loud(8)];
    v.shrink_to_fit();
    v.reserve(4);
    let popped = v.pop_if(|l| l.0 == 8);
    println!("popped {:?}", popped.map(|l| l.0));
    let none = v.pop_if(|l| l.0 == 0);
    println!("none {}", none.is_none());
    v.shrink_to(1);
    let mut q: VecDeque<Loud> = VecDeque::from([Loud(9), Loud(10)]);
    let front = q.pop_front_if(|l| l.0 == 9);
    println!("front {:?}", front.as_ref().map(|l| l.0));
    let mut o = Some(Loud(11));
    if let Some(l) = o.as_mut() {
        l.0 += 1;
    }
    println!("as_mut {:?}", o.as_ref().map(|l| l.0));
}
fn main() {
    mapped();
    defaults();
    popped();
    println!("end");
}
