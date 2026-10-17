// A vector's `drain` of values with destructors: what it takes out is an
// iterator that owns them, as `into_iter()`'s is, whose drop drops what it
// hasn't given yet, in order, as Rust's does (ADR 0098): all of it, of one
// dropped as it's made, none, of one collected.
struct N(u32);
impl Drop for N {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn ids(v: &[N]) -> Vec<u32> {
    v.iter().map(|n| n.0).collect()
}
fn main() {
    let mut v: Vec<N> = (1..=8).map(N).collect();
    v.drain(6..);
    println!("after statement {:?}", ids(&v));
    let taken: Vec<N> = v.drain(..2).collect();
    println!("taken {:?} left {:?}", ids(&taken), ids(&v));
    for n in v.drain(1..) {
        println!("got {}", n.0);
        if n.0 == 4 {
            break;
        }
    }
    println!("after loop {:?}", ids(&v));
    let mut w: Vec<N> = (10..14).map(N).collect();
    let mut d = w.drain(..);
    let first = d.next();
    println!("first {:?}", first.as_ref().map(|n| n.0));
    drop(d);
    println!("after drain {}", w.len());
    drop(first);
    println!("end");
}
