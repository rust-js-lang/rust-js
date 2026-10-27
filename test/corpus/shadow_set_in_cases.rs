// A `let` that shadows a binding with a `match` of constants, a `switch`,
// is the binding, set in its cases, where nothing names the old one once
// it may be set (ADR 0650, amended): React's scheduler's
// `unstable_runWithPriority`, `switch (priorityLevel) { case 1: .. break;
// default: priorityLevel = 3; }`, with a guarded case that falls through,
// and one set again in a loop after. One whose old value a closure made
// before reads, or a drop after, stays apart. And of an `Option`, whose arm
// gives what's in it after doing more: React's createWorkInProgress, `let
// workInProgress = current.alternate; if (workInProgress === null) {
// workInProgress = ..; } else { workInProgress.flags = NoFlags; }`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct Noisy(u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn run_with_priority(priority_level: u8) -> u8 {
    let priority_level = match priority_level {
        1 | 2 | 3 | 4 | 5 => priority_level,
        _ => 3,
    };
    priority_level * 10
}

fn guarded(level: u32, strict: bool) -> u32 {
    let level = match level {
        1 if strict => level + 100,
        1 | 2 | 3 => level,
        _ => 0,
    };
    level + 1
}

// A case that declares the name itself: its own, and the shadow apart.
fn named_in_a_case(level: u32) -> u32 {
    let level = match level {
        1 | 2 | 3 => level,
        4 | 5 | 6 => {
            let level = 7;
            level + 1
        }
        _ => 0,
    };
    level
}

// Of a range, an `if` chain: one whose branch is the binding's own value
// stays apart, not an empty branch.
fn ranged(level: u32) -> u32 {
    let level = match level {
        1..=3 => level,
        4 | 5 | 6 => {
            let tripled = level * 3;
            tripled + 1
        }
        _ => 0,
    };
    level
}

// Set again in a loop after, which names nothing else: the binding still.
fn set_again(level: u32) -> u32 {
    let mut level = match level {
        1 | 2 | 3 => level,
        _ => 0,
    };
    for step in 0..2 {
        level = level * 2 + step;
    }
    level
}

fn read_after(level: u8) -> (u8, u8) {
    let old = move || level;
    let level = match level {
        1 | 2 | 3 => level + 1,
        _ => 0,
    };
    (old(), level)
}

fn dropped_after(n: u32) {
    let noisy = Noisy(n);
    let noisy = match n {
        1 | 2 | 3 => Noisy(n * 10),
        _ => Noisy(0),
    };
    println!("made {}", noisy.0);
}

struct Fiber {
    flags: Cell<u32>,
    alternate: RefCell<Option<Rc<Fiber>>>,
}

fn work_in_progress(current: &Rc<Fiber>) -> Rc<Fiber> {
    let work_in_progress = current.alternate.borrow().clone();
    let work_in_progress = match work_in_progress {
        None => {
            let work_in_progress = Rc::new(Fiber {
                flags: Cell::new(1),
                alternate: RefCell::new(Some(Rc::clone(current))),
            });
            current.alternate.replace(Some(Rc::clone(&work_in_progress)));
            work_in_progress
        }
        Some(work_in_progress) => {
            work_in_progress.flags.set(0);
            work_in_progress
        }
    };
    work_in_progress
        .flags
        .set(work_in_progress.flags.get() + current.flags.get());
    work_in_progress
}

fn main() {
    for p in [0, 1, 3, 5, 6, 200] {
        println!("{}", run_with_priority(p));
    }
    for (level, strict) in [(1, true), (1, false), (3, true), (9, false)] {
        println!("{}", guarded(level, strict));
    }
    for level in [0, 2, 5, 7] {
        println!("{}", named_in_a_case(level));
    }
    for level in [0, 2, 5, 7] {
        println!("{}", ranged(level));
    }
    for level in [0, 2, 7] {
        println!("{}", set_again(level));
    }
    for level in [0, 1, 3, 9] {
        println!("{:?}", read_after(level));
    }
    dropped_after(2);
    dropped_after(7);
    let current = Rc::new(Fiber {
        flags: Cell::new(4),
        alternate: RefCell::new(None),
    });
    let first = work_in_progress(&current);
    println!("{}", first.flags.get());
    first.flags.set(9);
    let second = work_in_progress(&current);
    println!("{} {}", second.flags.get(), Rc::ptr_eq(&first, &second));
}
