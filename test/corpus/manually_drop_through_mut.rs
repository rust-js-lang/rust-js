// A `ManuallyDrop` is the value it holds (ADR 0406), and a `&mut` to one is
// a `&mut` to that value: of a number, the cell its `DerefMut` gives,
// written through and read back; of a struct, the struct, changed in place.
use std::mem::ManuallyDrop;

struct Counter {
    hits: u32,
}

fn bump(n: &mut ManuallyDrop<u32>) {
    **n += 1;
}

fn hit(counter: &mut ManuallyDrop<Counter>) {
    counter.hits += 2;
}

fn main() {
    let mut count = ManuallyDrop::new(5u32);
    bump(&mut count);
    let r: &mut u32 = &mut count;
    *r *= 3;
    println!("{}", *count);

    let mut counter = ManuallyDrop::new(Counter { hits: 1 });
    hit(&mut counter);
    hit(&mut counter);
    println!("{}", counter.hits);
    let inner = ManuallyDrop::into_inner(counter);
    println!("{}", inner.hits);
}
