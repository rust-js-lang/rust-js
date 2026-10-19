// `take(n)` or `skip(n)` of an iterator that owns items with destructors,
// moved into it: what it hasn't given is dropped when it ends or is dropped,
// as Rust's is, and what `skip` passes over as it's first stepped. Its
// source is `$closing`, which JS's `take` closes, or `$skipDropping`
// (ADR 0422). Of a lent one, `by_ref().take(n)`, what it hasn't given is
// its lender's, stepped on after (ADR 0468).
struct N(u32);
impl Drop for N {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn split_first<I: Iterator>(mut iter: I, k: usize) -> (Vec<I::Item>, usize) {
    let first: Vec<I::Item> = iter.by_ref().take(k).collect();
    (first, iter.count())
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
    let s: Vec<N> = (40..=44).map(N).collect();
    for n in s.into_iter().skip(2) {
        println!("after skip {}", n.0);
        if n.0 == 43 {
            break;
        }
    }
    let unstepped: Vec<N> = (50..=52).map(N).collect();
    let skipped = unstepped.into_iter().skip(1);
    println!("made skip");
    drop(skipped);
    let both: Vec<N> = (60..=64).map(N).collect();
    let kept: Vec<N> = both.into_iter().skip(1).take(2).collect();
    println!("kept {}", kept.len());
    let mut made = (70..=74).map(N);
    let lent: Vec<N> = made.by_ref().take(2).collect();
    println!("lent {}", lent.len());
    let next = made.next();
    println!("next {:?}", next.as_ref().map(|n| n.0));
    drop(next);
    drop(lent);
    let (first, rest) = split_first((80..=83).map(N).collect::<Vec<N>>().into_iter(), 1);
    println!("first {} rest {rest}", first.len());
    let mut owned = (90..=93).map(N).collect::<Vec<N>>().into_iter();
    for n in owned.by_ref().take(2) {
        println!("lent got {}", n.0);
    }
    println!("left {}", owned.count());
    println!("end");
}
