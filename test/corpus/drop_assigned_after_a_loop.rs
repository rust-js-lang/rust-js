// A variable given a new value each time round a loop, after an inner
// loop whose bound may panic, of a call that may panic too: the old value
// is dropped where it's assigned, after the new one is made and before
// it's in the variable, and the new one at the end. A panic of either
// call drops the old one, so a `finally` that drops it is only around
// what's before the assignment, never the assignment itself (#463).
use std::cell::RefCell;
use std::rc::Rc;

struct Noisy(u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn steps(round: u32) -> u32 {
    if round > 10 {
        panic!("too many rounds");
    }
    round % 2
}

fn label(round: u32) -> u32 {
    if round > 20 {
        panic!("no label");
    }
    20 + round
}

fn replaced(rounds: u32) {
    let mut held = Noisy(10);
    for round in 0..rounds {
        for step in 0..steps(round) {
            println!("step {round} {step}");
        }
        held = Noisy(label(round));
    }
    println!("held {}", held.0);
}

fn shared(rounds: u32) -> i64 {
    let mut cell = Rc::new(RefCell::new(-1));
    let first = Rc::clone(&cell);
    for round in 0..rounds {
        for step in 0..steps(round) {
            *cell.borrow_mut() += i64::from(step);
        }
        cell = Rc::new(RefCell::new(i64::from(label(round))));
    }
    println!("first {} count {}", first.borrow(), Rc::strong_count(&first));
    let last = *cell.borrow();
    last
}

fn main() {
    replaced(3);
    replaced(0);
    println!("shared {}", shared(3));
    let unwound = std::panic::catch_unwind(|| replaced(12));
    println!("unwound {}", unwound.is_err());
}
