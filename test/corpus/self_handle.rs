// A closure that refers to itself, as React's createEventHandle's handle
// adds itself to the target's set of handles: the `Rc` of a `RefCell` of
// the `Weak` its own `Rc` was made of, set right after, as the Rust book
// teaches. The closure runs only borrowed from its `Rc`, and the cell is
// set before anything can call it, so each upgrade gives that `Rc`: where
// nothing counts it, the cell is no JS and the upgrade is the closure's
// own name, `const handle = (n) => { .. handle .. }` (ADR 0843, amended).
// One whose counts the program reads, or whose cell it reads apart, is
// counted, as before (`self_handle_counted`).

use std::cell::RefCell;
use std::rc::{Rc, Weak};

type Countdown = dyn Fn(u32) -> u32;

// Counts down by calling itself through its handle.
fn countdown(step: u32) -> Rc<Countdown> {
    let this: Rc<RefCell<Option<Weak<Countdown>>>> = Rc::new(RefCell::new(None));
    let countdown: Rc<Countdown> = Rc::new({
        let this = Rc::clone(&this);
        move |n| {
            if n < step {
                return n;
            }
            let countdown = this.borrow().as_ref().and_then(Weak::upgrade).unwrap();
            1 + countdown(n - step)
        }
    });
    *this.borrow_mut() = Some(Rc::downgrade(&countdown));
    countdown
}

type Handle = dyn Fn(&str) -> bool;

// Adds itself to a list once, as React's handle adds itself to a target's
// set of handles, told apart from the others by its identity.
fn handle(seen: Rc<RefCell<Vec<Rc<Handle>>>>, log: Rc<RefCell<Vec<String>>>) -> Rc<Handle> {
    let this_handle = Rc::new(RefCell::new(None));
    let handle: Rc<Handle> = Rc::new({
        let this_handle = Rc::clone(&this_handle);
        move |target: &str| {
            let handle: Rc<Handle> = this_handle.borrow().as_ref().and_then(Weak::upgrade).unwrap();
            let added = !seen.borrow().iter().any(|h| Rc::ptr_eq(h, &handle));
            if added {
                seen.borrow_mut().push(handle);
            }
            log.borrow_mut().push(format!("{target} {added}"));
            added
        }
    });
    *this_handle.borrow_mut() = Some(Rc::downgrade(&handle));
    handle
}

fn main() {
    let down = countdown(3);
    println!("{} {} {}", down(0), down(7), down(10));

    let seen = Rc::new(RefCell::new(Vec::new()));
    let log = Rc::new(RefCell::new(Vec::new()));
    let first = handle(Rc::clone(&seen), Rc::clone(&log));
    let second = handle(Rc::clone(&seen), Rc::clone(&log));
    println!("{} {} {}", first("a"), first("b"), second("c"));
    println!("{:?} {}", log.borrow(), seen.borrow().len());
    let again = Rc::clone(&first);
    println!("{} {}", again("d"), Rc::ptr_eq(&again, &first));
}
