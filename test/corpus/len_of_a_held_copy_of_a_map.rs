// `len()` of a `copied()` or `cloned()` of a slice's `map`, held in a
// variable, reversed or not: the copy is its map in JS, made in place, whose
// type says nothing of its form. Its length asked of the variable, the map
// is made knowing it where it's written, `$mapEnds(..)`; a `rev()` a
// variable holds, std's struct, asks it of what it holds by its type, which
// is made knowing its back and length by type (#1978). Then stepped from
// either end, its length alone asked, its hint asked, given to generic
// code, and of items with a destructor, cloned, collected and dropped.
fn count<I: ExactSizeIterator>(it: I) -> usize {
    it.len()
}
fn hint<I: Iterator>(it: &I) -> (usize, Option<usize>) {
    it.size_hint()
}
#[derive(Clone, Debug)]
struct Loud(u32);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn main() {
    let items = [1, 2, 3, 4, 5];
    let m = items.iter().map(|x| x.min(&9)).copied().rev();
    println!("a {}", m.len());
    let mut m = items.iter().map(|x| x.min(&9)).copied().rev();
    println!("b {:?} {} {:?}", m.next(), m.len(), m.next_back());
    println!("c {}", m.len());
    let m = items.iter().map(|x| x.min(&9)).copied();
    println!("d {} {:?}", m.len(), m.size_hint());
    let mut m = items.iter().map(|x| x.min(&9)).cloned();
    println!("e {:?} {} {:?}", m.next_back(), m.len(), hint(&m));
    let m = items.iter().map(|x| x.min(&9)).copied().rev();
    println!("f {}", count(m));
    let m = items.iter().map(|x| x.min(&9)).copied();
    println!("g {:?} {}", m.size_hint(), count(m));
    let only = items.iter().map(|x| x.max(&2)).copied();
    println!("l {}", only.len());
    let only = items.iter().map(|x| x.max(&4)).cloned();
    println!("m {}", only.len());
    let louds = vec![Loud(1), Loud(2), Loud(3)];
    let m = louds.iter().map(|l| l).cloned().rev();
    println!("h {}", m.len());
    let v: Vec<Loud> = m.collect();
    println!("i {:?}", v);
    let mut m = louds.iter().map(|l| l).cloned();
    println!("j {} {:?}", m.len(), m.next_back());
    println!("k {}", m.len());
}
