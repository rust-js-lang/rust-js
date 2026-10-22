// What gives an `Option` of what could look like `None`, `()` or an
// `Option` itself: its `Some` boxed, or the value as it's given, boxed
// already.
use std::collections::BTreeSet;

struct Slot {
    held: Option<Option<i32>>,
}

fn main() {
    let unit: Result<i32, ()> = Err(());
    let none: Result<i32, Option<i32>> = Err(None);
    println!("{:?} {:?} {:?}", unit.err(), none.clone().err(), none.err().flatten());

    let u: Option<()> = Some(());
    println!("{:?} {:?}", u.xor(None), u.xor(u));

    let mut slot = Slot { held: Some(None) };
    let old = std::mem::replace(&mut slot.held, Some(Some(1)));
    let taken = std::mem::take(&mut slot.held);
    println!("{old:?} {taken:?} {:?}", slot.held);

    let n: Option<i32> = Some(3);
    let mapped: Option<Option<i32>> = n.map_or(Some(Some(9)), |_| Some(None));
    println!("{mapped:?}");

    let items = vec![None, Some(2), None, Some(4)];
    println!("{:?} {:?}", items.iter().copied().nth(2), items.iter().copied().nth(9));
    let mut it = items.into_iter().peekable();
    let first = it.next_if(|x| x.is_none());
    let second = it.next_if(|x| x.is_none());
    println!("{first:?} {second:?} {:?}", it.next());

    let set: BTreeSet<Option<i32>> = [Some(2), None].into_iter().collect();
    println!("{:?} {:?}", set.first(), set.last());
}
