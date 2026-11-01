// A borrow held while code runs that borrows only other cells of its type
// (rd482): a cell's flow (ADR 0863, amended) says which cells each borrow
// can be of, so a flattened state read while another chunk is written, or
// a `RefCell` field's items walked while another's are pushed to, is held
// as it's written, never checked, where the code it runs can't reach it.
// One the code it runs does reach, through a callback, is still counted:
// its `try_borrow_mut` finds it borrowed, as Rust's does.
use std::cell::RefCell;

pub struct Queue {
    pub items: RefCell<Vec<u32>>,
}

fn push(queue: &Queue, item: u32) {
    queue.items.borrow_mut().push(item);
}

// Each of `from`'s items given to `to` while `from`'s are borrowed, by a
// call of the crate's and by a borrow of `to`'s.
fn drain_into(from: &Queue, to: &Queue) {
    for &item in from.items.borrow().iter() {
        push(to, item * 10);
    }
}

fn copy_into(from: &Queue, to: &Queue) {
    for &item in from.items.borrow().iter() {
        to.items.borrow_mut().push(item + 1);
    }
}

thread_local! {
    static PENDING: Queue = const {
        Queue {
            items: RefCell::new(Vec::new()),
        }
    };
}

// A thread-local's queue walked while a callback runs, which code rust-js
// can't see calls, the callback reaching the queue by its key, not by
// anything it's given.
fn each_pending(f: &dyn Fn(u32) -> bool) -> Vec<bool> {
    PENDING.with(|pending| pending.items.borrow().iter().map(|&item| f(item)).collect())
}

pub struct Watched {
    pub items: RefCell<Vec<u32>>,
}

// A callback that may borrow what's walked: each item's, here the same
// cell's, which finds it borrowed.
fn each(watched: &Watched, f: &dyn Fn(u32) -> bool) -> Vec<bool> {
    watched.items.borrow().iter().map(|&item| f(item)).collect()
}

fn main() {
    let first = Queue {
        items: RefCell::new(vec![1, 2, 3]),
    };
    let second = Queue {
        items: RefCell::new(Vec::new()),
    };
    drain_into(&first, &second);
    let third = Queue {
        items: RefCell::new(Vec::new()),
    };
    copy_into(&first, &third);
    println!("{:?} {:?} {:?}", first.items.borrow(), second.items.borrow(), third.items.borrow());

    PENDING.with(|pending| pending.items.borrow_mut().extend([6, 7]));
    let queued = each_pending(&|item| {
        PENDING.with(|pending| match pending.items.try_borrow_mut() {
            Ok(mut items) => {
                items.push(item);
                true
            }
            Err(_) => false,
        })
    });
    println!("{queued:?}");

    let watched = Watched {
        items: RefCell::new(vec![4, 5]),
    };
    let tried = each(&watched, &|item| match watched.items.try_borrow_mut() {
        Ok(mut items) => {
            items.push(item);
            true
        }
        Err(_) => false,
    });
    println!("{tried:?} {:?}", watched.items.borrow());
}
