// A `for` over a `RefCell`'s borrow holds the borrow for the whole loop and
// drops it as the loop ends or is left: one `try { for .. } finally {
// $unborrow(..) }` (rd191). A callback that borrows the cell mutably
// meanwhile panics, as Rust's does; one that only reads goes on.

use std::cell::RefCell;
use std::panic::{self, AssertUnwindSafe};

struct List {
    items: RefCell<Vec<u32>>,
}

impl List {
    fn find_last(&self, f: &dyn Fn(u32) -> bool) -> Option<u32> {
        for &item in self.items.borrow().iter().rev() {
            if f(item) {
                return Some(item);
            }
        }
        None
    }

    // A range over the borrow, which it changes: React Float's
    // hydrateHoistable, its `for (let i = 0; ..)` in one `try` (rd200).
    fn take_first(&self, f: &dyn Fn(u32) -> bool) -> Option<u32> {
        let mut items = self.items.borrow_mut();
        for i in 0..items.len() {
            if !f(items[i]) {
                continue;
            }
            return Some(items.remove(i));
        }
        None
    }

    fn any(&self, f: &dyn Fn(u32) -> bool) -> bool {
        for &item in self.items.borrow().iter() {
            if f(item) {
                return true;
            }
        }
        false
    }
}

struct Noisy(u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

// Of a value one way out moves, `return held`, the other drops: no `finally`
// drops it on the way that moves it.
fn kept_or_dropped(stop: u32) -> Option<Noisy> {
    let mut n = 0;
    {
        let held = Noisy(stop);
        loop {
            n += 1;
            if n == stop {
                return Some(held);
            }
            if n > 3 {
                break;
            }
        }
    }
    println!("after {n}");
    None
}

fn main() {
    println!("{}", kept_or_dropped(2).map_or(0, |held| held.0));
    println!("{}", kept_or_dropped(9).is_none());
    let list = List { items: RefCell::new(vec![1, 4, 2, 8, 5]) };
    println!("{:?}", list.find_last(&|x| x % 2 == 0));
    println!("{:?}", list.find_last(&|x| x > 9));
    println!("{}", list.any(&|x| x == 2));
    println!("{:?} {:?}", list.take_first(&|x| x == 8), list.take_first(&|x| x > 99));
    list.items.borrow_mut().push(8);
    println!("{}", list.any(&|x| x == 7 || list.items.borrow().len() > 9));
    list.items.borrow_mut().push(7);
    let pushed = panic::catch_unwind(AssertUnwindSafe(|| {
        list.any(&|x| {
            list.items.borrow_mut().push(x);
            false
        })
    }));
    println!("pushed while borrowed: {}", pushed.is_err());
    // Released as the panic left the loop: it's borrowed again.
    list.items.borrow_mut().push(9);
    println!("{:?}", list.items.borrow());
}
