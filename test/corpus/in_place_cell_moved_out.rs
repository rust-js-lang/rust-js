//@ library-differs
// What generic code moves out of an in-place cell, a dictionary's `&mut
// Self::Out` and `Cell::from_mut`'s, is apart from what's written next; a
// `Copy` struct `get()` of such a cell is copied, as its `set` changes the
// object in place; and a `&mut` a call gives, `h.get()`, is reached once
// by `mem::replace`, not by its read and its write (#527, ADR 0454).
use std::cell::Cell;
use std::mem;

#[derive(Debug, Clone, Copy, PartialEq)]
struct P {
    x: i32,
}

trait Get {
    type Out;
    fn get(&mut self) -> &mut Self::Out;
}

struct Holder(Vec<i32>);

impl Get for Holder {
    type Out = Vec<i32>;
    fn get(&mut self) -> &mut Vec<i32> {
        println!("get {:?}", self.0);
        &mut self.0
    }
}

fn reset<H: Get>(h: &mut H, v: H::Out) -> H::Out {
    mem::replace(h.get(), v)
}

fn swap_out<H: Get>(a: &mut H, b: &mut H) {
    mem::swap(a.get(), b.get());
}

fn cell_replace_generic<T>(c: &Cell<T>, v: T) -> T {
    c.replace(v)
}

fn cell_swap_generic<T>(a: &Cell<T>, b: &Cell<T>) {
    a.swap(b)
}

fn main() {
    let mut p = P { x: 1 };
    let c = Cell::from_mut(&mut p);
    let got = c.get();
    c.set(P { x: 2 });
    println!("{got:?} {p:?}");

    let mut h = Holder(vec![1, 2]);
    let old = reset(&mut h, vec![3]);
    println!("{old:?} {:?}", h.0);

    let mut a = Holder(vec![1]);
    let mut b = Holder(vec![2]);
    swap_out(&mut a, &mut b);
    println!("{:?} {:?}", a.0, b.0);

    let mut v = vec![1, 2];
    let old = cell_replace_generic(Cell::from_mut(&mut v), vec![3]);
    println!("{old:?} {v:?}");

    let mut v = vec![1, 2];
    let plain = Cell::new(vec![5]);
    cell_swap_generic(Cell::from_mut(&mut v), &plain);
    println!("{v:?} {:?}", plain.take());
}
