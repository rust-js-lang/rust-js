// What a `RefCell` of an object holds, read through its guard, compares as
// the value it is, not borrowed again: the cell is the object, mutably
// borrowed while it's compared, and only a comparison of the cells, or of
// one inside what's compared, borrows it (ADRs 0328, 0863).

use std::cell::RefCell;

#[derive(Debug, Clone, PartialEq)]
struct State {
    step: u32,
    log: Vec<String>,
}

fn advance(state: &RefCell<State>, done: &State) -> bool {
    let mut state = state.borrow_mut();
    state.step += 1;
    let line = format!("step {}", state.step);
    state.log.push(line);
    *state == *done
}

fn main() {
    let state = RefCell::new(State {
        step: 0,
        log: Vec::new(),
    });
    let done = State {
        step: 2,
        log: vec!["step 1".to_string(), "step 2".to_string()],
    };
    println!("{} {}", advance(&state, &done), advance(&state, &done));
    let mut held = state.borrow_mut();
    let same = *held == done;
    held.step = 3;
    println!("{same} {}", *held == done);
    drop(held);
    let outer = RefCell::new(vec![RefCell::new(1), RefCell::new(2)]);
    let inner = outer.borrow();
    let guard = inner[0].borrow_mut();
    let copy = vec![RefCell::new(1), RefCell::new(2)];
    let compared = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| *inner == copy));
    println!("{} {}", compared.is_err(), *guard);
}
