// A Vec of a slice, its items cloned, of a deque, and of text, its bytes,
// by From and Into.
use std::collections::VecDeque;
#[derive(Debug, Clone)]
struct P(String);
fn main() {
    let a = [1, 2, 3];
    let v = Vec::from(&a[1..]);
    let mut m = [5u8, 6];
    let w = Vec::from(&mut m[..]);
    let s: Vec<u8> = "hé".into();
    let t: Vec<u8> = Vec::from(String::from("ok"));
    let d: VecDeque<i32> = (1..4).collect();
    let mut d2 = d.clone();
    d2.push_front(0);
    let x: Vec<i32> = d2.into();
    let ps = [P("a".into())];
    let mut owned = Vec::from(&ps[..]);
    owned[0].0.push('!');
    println!("{v:?} {w:?} {s:?} {t:?} {x:?} {owned:?} {ps:?} {d:?}");
    let y = Vec::from(d);
    println!("{y:?}");
}
