// `iter::once_with(f)` (#1373): `f` is called when the iterator is first
// stepped, not where it's made, and its value is the one item; an iterator
// never stepped never calls it. It's `TrustedLen`, so a `Vec` it's collected
// into has room for exactly its one item.
use std::collections::HashSet;
use std::iter;

fn main() {
    let mut it = iter::once_with(|| {
        println!("called");
        5
    });
    println!("made");
    println!("{:?}", it.next());
    println!("{:?}", it.next());

    let _never = iter::once_with(|| {
        println!("never");
        1
    });
    let skipped: Vec<i32> = iter::once_with(|| {
        println!("skipped");
        2
    })
    .take(0)
    .collect();
    println!("{skipped:?}");

    let v: Vec<String> = iter::once_with(|| String::from("a")).chain(iter::once(String::from("b"))).collect();
    println!("{v:?}");
    let one: Vec<u8> = iter::once_with(|| 7).collect();
    println!("{one:?} {}", one.capacity());

    let mut seen = 0;
    let n = iter::once_with(|| {
        seen += 1;
        seen
    })
    .count();
    println!("{n} {seen}");
    println!("{:?}", iter::once_with(|| 4).last());
    println!("{}", iter::once_with(|| 6).sum::<i32>());

    let mut text = String::new();
    text.extend(iter::once_with(|| 'q'));
    let set: HashSet<u32> = iter::once_with(|| 9).collect();
    println!("{text} {set:?}");
    let mut peek = iter::once_with(|| 3).peekable();
    println!("{:?}", peek.peek());
    println!("{:?}", peek.next());
    println!("{:?}", peek.peek());
}
