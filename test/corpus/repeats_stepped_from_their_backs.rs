// `repeat_n(x, n)` and `repeat(x).take(n)` stepped from their backs, as
// std's `RepeatN` and `Take<Repeat<T>>` step them, each back step their
// front step, cloning and dropping what those do, and their `len()`; a
// `take` of a `repeat` held in a variable, std's struct, its `next_back`
// reading the value its `Repeat` holds; a `repeat`'s `clone()` and
// `cycle()`, which clones it once. Each was refused, or threw (#1139).
use std::iter::{repeat, repeat_n};

#[derive(Debug)]
struct Loud(u32);

impl Clone for Loud {
    fn clone(&self) -> Self {
        println!("clone {}", self.0);
        Loud(self.0 + 10)
    }
}

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn takes() {
    let v: Vec<u8> = repeat(7).take(3).rev().collect();
    println!("{v:?}");
    let louds: Vec<Loud> = repeat(Loud(1)).take(2).rev().collect();
    println!("{louds:?}");
    drop(louds);
    let mut held = repeat('x').take(4);
    println!("{:?} {}", held.next_back(), held.len());
    println!("{:?} {:?} {}", held.nth_back(1), held.next(), held.len());
    println!("{:?}", held.next_back());
    let mut louder = repeat(Loud(2)).take(3);
    println!("{:?}", louder.nth_back(1));
    println!("{}", louder.len());
    let folded = repeat(3u32).take(4).rfold(0, |a, x| a * 10 + x);
    println!("{folded}");
    let found = repeat(5u8).take(3).rposition(|x| x == 5);
    println!("{found:?}");
    println!("end takes");
}

fn ns() {
    let v: Vec<String> = repeat_n(String::from("a"), 3).rev().collect();
    println!("{v:?}");
    let mut r = repeat_n(5u32, 4);
    println!("{} {:?} {}", r.len(), r.nth_back(1), r.len());
    println!("{:?} {} {:?}", r.next_back(), r.len(), r.next_back());
    println!("{:?}", repeat_n(1, 0).next_back());
    let mut louds = repeat_n(Loud(3), 3);
    println!("{:?}", louds.next_back());
    println!("{}", louds.len());
    println!("{:?}", louds.nth_back(5));
    println!("{}", louds.len());
    let none = repeat_n(Loud(4), 0).rev().count();
    println!("{none}");
    let shown = repeat_n(Loud(5), 2).rfold(String::new(), |s, l| format!("{s}{l:?};"));
    println!("{shown}");
    println!("{:?}", repeat_n('y', 3).rev().rposition(|c| c == 'y'));
    println!("{:?}", repeat_n(Loud(8), 3).rev().nth(1));
    println!("end ns");
}

fn cycles() {
    let v: Vec<u8> = repeat(3).cycle().take(4).collect();
    println!("{v:?}");
    let louds: Vec<Loud> = repeat(Loud(6)).cycle().take(2).collect();
    println!("{louds:?}");
    drop(louds);
    let r = repeat(Loud(7));
    let mut c = r.clone();
    println!("{:?}", c.next());
    drop(r);
    println!("end cycles");
}

fn main() {
    takes();
    ns();
    cycles();
}
