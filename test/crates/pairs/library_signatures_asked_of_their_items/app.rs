//! Each as Rust runs it.
use dep::{front, make};

pub fn main() {
    let mut outer = vec![vec![1, 2], vec![3]].into_iter();
    let mut slot = None;
    let a = front(&mut outer, &mut slot, |inner| inner.next());
    let b = front(&mut outer, &mut slot, |inner| inner.next());
    let c = front(&mut outer, &mut slot, |inner| inner.next());
    println!("{a:?} {b:?} {c:?} {:?}", slot.map(|inner| inner.len()));
    let made = make(|add| {
        add("a");
        add("b");
    });
    println!("{}", made.iter().map(|f| f("x")).collect::<Vec<_>>().join(", "));
}
