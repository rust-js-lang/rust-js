// A `Vec` extended by an adapter of JS's own iterator a variable holds,
// `let d = v.iter().map(f); w.extend(d.map(g))`: the variable keeps that
// its form is JS's own (`Known::IterHintless`), whose fold is the steps its
// `next` takes, so the extend is by its steps, `$extend(w, d.map(g))`, not
// given the `true` that asks its fold, which changes nothing (#1803, #1792).
// A slice's and a B-tree set's items, mapped twice; items with a
// destructor, dropped as they're taken.
use std::collections::BTreeSet;

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop Loud {}", self.0);
    }
}

fn double(x: &u32) -> u32 {
    x * 2
}

fn of_slice(v: &[u32]) -> Vec<u32> {
    let d = v.iter().map(double);
    let mut w = vec![0];
    w.extend(d.map(|x| x + 1));
    w
}

fn of_set(s: &BTreeSet<u32>) -> Vec<u32> {
    let d = s.iter().map(double);
    let mut w = vec![0];
    w.extend(d.map(|x| x + 1));
    w
}

fn of_louds(louds: &[Loud]) -> Vec<Loud> {
    let d = louds.iter().map(|l| Loud(l.0 * 10));
    let mut w = vec![Loud(0)];
    w.extend(d.map(|l| Loud(l.0 + 1)));
    w
}

fn main() {
    let s: BTreeSet<u32> = [3, 1, 2].into_iter().collect();
    println!("{:?} {:?}", of_slice(&[1, 2, 3]), of_set(&s));
    println!("{:?}", of_slice(&[]));
    let louds = [Loud(1), Loud(2)];
    let w = of_louds(&louds);
    println!("{:?}", w.iter().map(|l| l.0).collect::<Vec<_>>());
}
