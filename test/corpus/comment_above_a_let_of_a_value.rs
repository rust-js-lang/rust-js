// A comment above a `let` whose value opens, `let listeners = match .. {`
// or `let f = {`, is above the whole `let`'s first JS: the `let
// listeners;` the `match`'s `if` sets, in the `try` of the `catch_unwind`
// closure it's written in, as React's initializeModelChunk has it (#758);
// and the block's first statement, `const g = [5];`, of a block whose
// closure holds a destructor and is dropped by `drop(f)` (#767). Each
// keeps the blank line the Rust has above it, and the `=` of a type,
// `Iterator<Item = u32>`, is the type's.
use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};

pub enum State {
    Blocked { value: Option<u32> },
    Done,
}

fn revive(n: u32) -> u32 {
    if n > 100 {
        panic!("too big");
    }
    n + 1
}

pub fn init(state: &RefCell<State>, n: u32, log: &mut Vec<u32>) {
    log.push(0);
    if catch_unwind(AssertUnwindSafe(|| {
        let value = revive(n);

        // Invoke any listeners added while resolving this model. I.e. cyclic
        // references.
        let listeners = match &mut *state.borrow_mut() {
            State::Blocked { value, .. } => value.take(),
            _ => None,
        };
        log.push(value + listeners.unwrap_or(0));
    }))
    .is_err()
    {
        log.push(9);
    }
}

pub fn outside(state: &RefCell<State>, log: &mut Vec<u32>) {
    log.push(1);

    // A `let` of a `match` outside a `try`.
    let taken = match &mut *state.borrow_mut() {
        State::Blocked { value, .. } => value.take(),
        State::Done => Some(7),
    };
    log.push(taken.unwrap_or(3));
}

pub fn typed(state: &RefCell<State>, log: &mut Vec<u32>) {
    log.push(2);

    // A `let` whose type has an `=` of its own.
    let taken: Option<Box<dyn Iterator<Item = u32>>> = match &mut *state.borrow_mut() {
        State::Blocked { value, .. } => value
            .take()
            .map(|v| Box::new(std::iter::once(v)) as Box<dyn Iterator<Item = u32>>),
        State::Done => None,
    };
    log.extend(taken.into_iter().flatten());
}

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn dropped() {
    println!("start");

    // A comment above a let of a block.
    let f = {
        let g = D(5);
        move || {
            let _g = &g;
            g.0
        }
    };
    println!("f {}", f());
    drop(f);

    // A comment above a let.
    let g = D(6);
    let f = move || {
        let _g = &g;
        g.0
    };
    println!("f {}", f());
    drop(f);
}

fn main() {
    let mut log = Vec::new();
    let state = RefCell::new(State::Blocked { value: Some(4) });
    init(&state, 2, &mut log);
    init(&state, 3, &mut log);
    init(&RefCell::new(State::Done), 5, &mut log);
    outside(&RefCell::new(State::Blocked { value: Some(8) }), &mut log);
    outside(&RefCell::new(State::Done), &mut log);
    typed(&RefCell::new(State::Blocked { value: Some(6) }), &mut log);
    typed(&RefCell::new(State::Done), &mut log);
    println!("{log:?}");
    dropped();
}
