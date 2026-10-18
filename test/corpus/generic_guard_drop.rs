// A guard given to generic code is dropped there, its borrow given back,
// as any value whose drop does something is; and what's read through a
// reference into a value is read before the value's dropped (ADR 0451).
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

fn take<T>(value: T) -> usize {
    let _ = &value;
    1
}

fn length<D: Deref<Target = Vec<u32>>>(items: D) -> usize {
    items.len()
}

fn shown<D: Deref<Target = Vec<u32>>>(items: D) {
    println!("{} items", items.len());
}

fn first_or<D: Deref<Target = Vec<u32>>>(items: D, or: u32) -> u32 {
    let first = items.first().copied();
    first.unwrap_or(or)
}

struct Holder {
    items: Vec<u32>,
    log: Rc<RefCell<Vec<&'static str>>>,
}

impl Deref for Holder {
    type Target = Vec<u32>;

    fn deref(&self) -> &Vec<u32> {
        &self.items
    }
}

impl Drop for Holder {
    fn drop(&mut self) {
        self.items.clear();
        self.log.borrow_mut().push("dropped");
    }
}

fn main() {
    let cell = RefCell::new(vec![1, 2]);
    take(cell.borrow());
    println!("{}", length(cell.borrow()));
    cell.borrow_mut().push(3);
    println!("{}", length(cell.borrow_mut()));
    shown(cell.borrow());
    println!("{}", first_or(cell.borrow(), 0));
    let log = Rc::new(RefCell::new(Vec::new()));
    let held = Holder {
        items: vec![4, 5, 6],
        log: Rc::clone(&log),
    };
    println!("{} {:?}", length(held), log.borrow());
}
