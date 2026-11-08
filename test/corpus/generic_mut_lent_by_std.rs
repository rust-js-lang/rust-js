// A `&mut` to a type parameter's value that a std call lends generic code:
// a box, `{ value }`, as generic code reads every `&mut T` (ADR 0099), here
// of a number. Each gave the value itself, which a write through threw on
// in JS (#1007), and `dedup_by` and `pop_if` were refused.

use std::cell::{LazyCell, OnceCell};
use std::collections::{BinaryHeap, VecDeque};
use std::pin::Pin;
use std::rc::Rc;

fn option_as_mut<T>(o: &mut Option<T>, v: T) {
    if let Some(x) = o.as_mut() {
        *x = v;
    }
}

fn result_as_mut<T, E>(r: &mut Result<T, E>, v: T) {
    if let Ok(x) = r.as_mut() {
        *x = v;
    }
}

fn result_iter_mut<T, E>(r: &mut Result<T, E>, v: T) {
    if let Some(x) = r.iter_mut().next() {
        *x = v;
    }
}

fn pin_set<T>(mut p: Pin<&mut T>, v: T) {
    p.set(v);
}

fn pin_as_mut<T: Unpin>(mut p: Pin<&mut T>, v: T) {
    *p.as_mut().get_mut() = v;
}

fn rc_get_mut<T>(r: &mut Rc<T>, v: T) {
    if let Some(x) = Rc::get_mut(r) {
        *x = v;
    }
}

fn once_get_mut<T>(c: &mut OnceCell<T>, v: T) {
    if let Some(x) = c.get_mut() {
        *x = v;
    }
}

fn lazy_force_mut<T, F: FnOnce() -> T>(c: &mut LazyCell<T, F>, v: T) {
    *LazyCell::force_mut(c) = v;
}

fn peek_mut<T, I: Iterator<Item = T>>(items: I, v: T) -> Vec<T> {
    let mut items = items.peekable();
    if let Some(x) = items.peek_mut() {
        *x = v;
    }
    items.collect()
}

fn dedup<T: PartialEq>(mut v: Vec<T>) -> Vec<T> {
    v.dedup_by(|a, b| a == b);
    v
}

fn pop_if<T: PartialEq>(v: &mut Vec<T>, t: T) -> Option<T> {
    v.pop_if(|x| *x == t)
}

fn heap_peek_mut<T: Ord>(h: &mut BinaryHeap<T>, v: T) {
    if let Some(mut x) = h.peek_mut() {
        *x = v;
    }
}

fn deque_range_mut<T: Copy>(d: &mut VecDeque<T>, v: T) {
    for x in d.range_mut(0..1) {
        *x = v;
    }
}

fn main() {
    let mut o = Some(1u32);
    option_as_mut(&mut o, 2);
    println!("{o:?}");
    let mut r: Result<u32, ()> = Ok(1);
    result_as_mut(&mut r, 3);
    println!("{r:?}");
    result_iter_mut(&mut r, 4);
    println!("{r:?}");
    let mut n = 1u32;
    pin_set(Pin::new(&mut n), 5);
    println!("{n}");
    pin_as_mut(Pin::new(&mut n), 6);
    println!("{n}");
    let mut rc = Rc::new(1u32);
    rc_get_mut(&mut rc, 7);
    println!("{rc}");
    let mut c = OnceCell::new();
    let _ = c.set(1u32);
    once_get_mut(&mut c, 8);
    println!("{c:?}");
    let mut l = LazyCell::new(|| 1u32);
    lazy_force_mut(&mut l, 9);
    println!("{}", *l);
    println!("{:?}", peek_mut([1u32, 2].into_iter(), 10));
    println!("{:?}", dedup(vec![1u32, 1, 2]));
    let mut v = vec![1u32, 2];
    println!("{:?} {v:?}", pop_if(&mut v, 2));
    let mut h = BinaryHeap::from(vec![1u32, 5]);
    heap_peek_mut(&mut h, 0);
    println!("{:?}", h.into_sorted_vec());
    let mut d = VecDeque::from(vec![1u32, 2]);
    deque_range_mut(&mut d, 11);
    println!("{d:?}");
    // And of a value JS changes in place, the box's `value` the object.
    let mut s = Some(String::from("a"));
    option_as_mut(&mut s, String::from("b"));
    println!("{s:?}");
    let mut words = Rc::new(vec![1]);
    rc_get_mut(&mut words, vec![2]);
    println!("{words:?}");
    let mut lists: Result<Vec<u8>, ()> = Ok(vec![1]);
    result_as_mut(&mut lists, vec![3]);
    println!("{lists:?}");
}
