// An element of a `RefCell<Vec<T>>` a field holds, reached through a guard
// a temporary holds, `root.lanes.borrow_mut()[index] |= lane`, is that
// element's place, `root.lanes[$at(root.lanes, index)]`, as React's
// ReactFiberLane writes `root.entanglements[index] |= lane`; and so is a
// `&mut` to an element at a parameter's index, `match &mut v[index]`, no
// getter and setter of it (ADR 0897). A `vec![..]` made while a guard is
// held borrows nothing. What's checked still fails where Rust's does: an
// index out of bounds, a borrow that meets another, and a place whose
// index or root is written again while it's borrowed is the one it was.
use std::cell::RefCell;
use std::panic::{self, AssertUnwindSafe};

struct Root {
    lanes: RefCell<Vec<u32>>,
    hidden: RefCell<Vec<Option<Vec<u32>>>>,
    counts: RefCell<Vec<u8>>,
}

fn root(n: usize) -> Root {
    Root {
        lanes: RefCell::new(vec![0; n]),
        hidden: RefCell::new(vec![None; n]),
        counts: RefCell::new(vec![0; n]),
    }
}

fn entangle(root: &Root, index: usize, lane: u32) {
    root.lanes.borrow_mut()[index] |= lane;
}

fn entangle_held(root: &Root, lanes: u32) {
    let mut held = root.lanes.borrow_mut();
    for index in 0..held.len() {
        if held[index] & lanes != 0 {
            held[index] |= lanes;
        }
    }
}

fn hide(root: &Root, index: usize, update: u32) {
    let mut hidden = root.hidden.borrow_mut();
    match &mut hidden[index] {
        None => hidden[index] = Some(vec![update]),
        Some(list) => list.push(update),
    }
}

fn hide_as_mut(root: &Root, index: usize, update: u32) {
    let mut hidden = root.hidden.borrow_mut();
    match hidden[index].as_mut() {
        None => hidden[index] = Some(vec![update, update]),
        Some(list) => list.push(update),
    }
}

fn bump(root: &Root, mut index: usize) -> usize {
    let mut lanes = root.lanes.borrow_mut();
    let first = &mut lanes[index];
    index += 1;
    *first += 10;
    index
}

fn bump_pushed(v: &mut Vec<u32>, index: usize) {
    v[index] += 100;
    v.push(0);
}

fn moved_root<'a>(mut root: &'a Root, other: &'a Root) {
    root.lanes.borrow_mut()[{
        root = other;
        0
    }] |= 4;
    root.lanes.borrow_mut()[1] |= 8;
}

fn count(root: &Root, index: usize) {
    root.counts.borrow_mut()[index] += 1;
}

fn meets(root: &Root) -> u8 {
    let held = root.counts.borrow_mut();
    count(root, 0);
    held[0]
}

fn main() {
    let r = root(4);
    entangle(&r, 2, 1);
    entangle(&r, 2, 2);
    entangle(&r, 1, 8);
    entangle_held(&r, 2);
    println!("{:?}", r.lanes.borrow());
    hide(&r, 3, 5);
    hide(&r, 3, 6);
    hide_as_mut(&r, 0, 7);
    hide_as_mut(&r, 0, 8);
    println!("{:?}", r.hidden.borrow());
    let next = bump(&r, 1);
    let mut v = vec![1, 2, 3];
    bump_pushed(&mut v, next);
    println!("{:?} {v:?} {next}", r.lanes.borrow());
    let other = root(2);
    moved_root(&r, &other);
    println!("{:?} {:?}", r.lanes.borrow(), other.lanes.borrow());
    let out = panic::catch_unwind(AssertUnwindSafe(|| entangle(&r, 9, 1)));
    println!("{}", out.is_err());
    let out = panic::catch_unwind(AssertUnwindSafe(|| hide(&r, 4, 1)));
    println!("{}", out.is_err());
    let out = panic::catch_unwind(AssertUnwindSafe(|| bump_pushed(&mut v, 4)));
    println!("{}", out.is_err());
    let out = panic::catch_unwind(AssertUnwindSafe(|| meets(&r)));
    count(&r, 1);
    println!("{} {:?}", out.is_err(), r.counts.borrow());
}
