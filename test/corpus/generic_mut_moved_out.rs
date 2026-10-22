// What generic code moves out of a `&mut T` it's given, `mem::replace`'s
// old value, `swap`'s other side, `take`'s, is a value of its own: the
// caller's object becomes the new value, the old one kept, of a struct, a
// `Vec`, a tuple, and through a trait's `&mut self` default.
use std::mem;

#[derive(Debug, Default, Clone)]
struct Stack {
    items: Vec<i32>,
}

trait Drain: Default {
    fn drain_out(&mut self) -> Self {
        mem::take(self)
    }
}

impl Drain for Stack {}

fn put<T>(slot: &mut T, value: T) -> T {
    mem::replace(slot, value)
}

fn exchange<T>(a: &mut T, b: &mut T) {
    mem::swap(a, b);
}

fn main() {
    let mut stack = Stack { items: vec![1] };
    let old = put(&mut stack, Stack { items: vec![2] });
    stack.items.push(3);
    println!("{stack:?} {old:?}");

    let mut a = vec![1];
    let mut b = vec![2, 3];
    exchange(&mut a, &mut b);
    a.push(4);
    println!("{a:?} {b:?}");

    let mut log = Vec::new();
    let mut slot = (0, vec![0]);
    for n in 1..4 {
        log.push(put(&mut slot, (n, vec![n])));
    }
    println!("{log:?} {slot:?}");

    let mut full = Stack { items: vec![7, 8] };
    let drained = full.drain_out();
    full.items.push(9);
    println!("{full:?} {drained:?}");
}
