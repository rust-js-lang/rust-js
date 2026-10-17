// `take(n)` of an iterator that owns items with destructors, moved into it:
// what it hasn't given is dropped when the `take` ends or is dropped, as
// Rust's is: its source is `$closing`, which JS's `take` closes (ADR 0422).
struct N(u32);
impl Drop for N {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn main() {
    let v: Vec<N> = (1..=5).map(N).collect();
    for n in v.into_iter().take(2) {
        println!("got {}", n.0);
    }
    println!("after loop");
    let w: Vec<N> = (6..=9).map(N).collect();
    for n in w.into_iter().take(3) {
        if n.0 == 7 {
            println!("break");
            break;
        }
        println!("got {}", n.0);
    }
    println!("after break");
    let x: Vec<N> = (10..=12).map(N).collect();
    let firsts: Vec<N> = x.into_iter().take(1).collect();
    println!("collected {}", firsts.len());
    let y: Vec<N> = (20..=22).map(N).collect();
    let mut t = y.into_iter().take(2);
    let one = t.next();
    println!("one {:?}", one.as_ref().map(|n| n.0));
    drop(t);
    println!("dropped take");
    drop(one);
    let z: Vec<N> = (30..=31).map(N).collect();
    let all: Vec<N> = z.into_iter().take(5).collect();
    println!("all {}", all.len());
    println!("end");
}
