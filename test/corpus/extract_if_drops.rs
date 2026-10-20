// `extract_if` of values with a destructor: what it takes out is its
// iterator's to give, each dropped where what it's given to drops it, and
// what it doesn't reach stays, dropped with the collection (ADR 0506).
use std::collections::{BTreeMap, HashMap, LinkedList};
struct Loud(u8);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn ids(v: &[Loud]) -> Vec<u8> {
    v.iter().map(|l| l.0).collect()
}
fn collected() {
    let mut v = vec![Loud(1), Loud(2), Loud(3), Loud(4), Loud(5)];
    let odd: Vec<Loud> = v.extract_if(.., |l| l.0 % 2 == 1).collect();
    println!("odd {:?} kept {:?}", ids(&odd), ids(&v));
}
fn stopped_early() {
    let mut w = vec![Loud(10), Loud(11), Loud(12), Loud(13)];
    let mut e = w.extract_if(1.., |_| true);
    let first = e.next();
    drop(e);
    println!("first {} left {:?}", first.map_or(0, |l| l.0), ids(&w));
}
fn counted() {
    let mut list: LinkedList<Loud> = [Loud(20), Loud(21)].into_iter().collect();
    let n = list.extract_if(|l| l.0 == 20).count();
    println!("counted {n} left {}", list.len());
}
fn from_maps() {
    let mut m: BTreeMap<u8, Loud> = [(1, Loud(30)), (2, Loud(31))].into_iter().collect();
    for (k, l) in m.extract_if(.., |k, _| *k == 2) {
        println!("map took {k} {}", l.0);
    }
    let mut h = HashMap::from([(1, Loud(40))]);
    let gone = h.extract_if(|_, l| l.0 == 40).count();
    println!("hash {gone} {} {}", h.len(), m.len());
}
fn main() {
    collected();
    stopped_early();
    counted();
    from_maps();
    println!("end");
}
