// A `&mut` a call gives back, reborrowed where it's bound: through an
// `unsafe` block, `let slots = unsafe { f(headers) };`, or by hand,
// `&mut *f(headers)`, and a `&mut` to the call's own `&mut`. The call runs
// once, its effects with it, and the binding is what it gave: the reborrow
// is never tried as an alias of a place in a way that drops the call
// (#1580, httparse's `deinit_slice_mut`).
use std::mem::MaybeUninit;

fn same<'a, 'b>(s: &'a mut &'b mut [u8]) -> &'a mut &'b mut [u8] {
    println!("same");
    s
}

unsafe fn deinit_slice_mut<'a, 'b, T>(s: &'a mut &'b mut [T]) -> &'a mut &'b mut [MaybeUninit<T>] {
    let s: *mut &mut [T] = s;
    let s = s as *mut &mut [MaybeUninit<T>];
    unsafe { &mut *s }
}

fn through_unsafe(headers: &mut &mut [u8]) -> usize {
    #[allow(unused_unsafe)]
    let slots = unsafe { same(headers) };
    slots[0] = 9;
    slots.len()
}

fn by_hand(headers: &mut &mut [u8]) -> usize {
    let slots = &mut *same(headers);
    slots[1] = 8;
    slots.len()
}

fn deinit(headers: &mut &mut [u8]) -> usize {
    let slots = unsafe { deinit_slice_mut(headers) };
    slots[2] = MaybeUninit::new(7);
    slots.len()
}

fn pushed(v: &mut Vec<u8>) -> &mut Vec<u8> {
    println!("pushed");
    v.push(4);
    v
}

struct Counter {
    n: u32,
}

fn bumped(c: &mut Counter) -> &mut Counter {
    println!("bumped");
    c.n += 1;
    c
}

fn main() {
    let mut storage = [1u8, 2, 3];
    let mut headers: &mut [u8] = &mut storage;
    let n = through_unsafe(&mut headers) + by_hand(&mut headers) + deinit(&mut headers);
    println!("{n} {storage:?}");

    let mut v = vec![1u8];
    let r: &mut &mut Vec<u8> = &mut pushed(&mut v);
    r.push(5);
    println!("{v:?}");

    let mut c = Counter { n: 1 };
    let held: &mut &mut Counter = &mut bumped(&mut c);
    held.n += 10;
    println!("{}", c.n);
}
