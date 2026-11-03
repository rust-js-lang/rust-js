// A `match` arm's binding of a `Cell` field, read once by `.get()`, is that
// field read where it's read, `return kind.current;`, as an arm that sets
// another field first reads it; one read twice is held once, `const
// current = kind.current;`, nothing between that could set it (#532, ADR
// 0287, amended).
use std::cell::Cell;

pub enum Kind {
    Signal { current: Cell<i32> },
    Computed { value: Cell<i32>, dirty: Cell<bool> },
}

fn read(kind: &Kind) -> i32 {
    match kind {
        Kind::Signal { current } => current.get(),
        Kind::Computed { value, .. } => value.get(),
    }
}

fn read_twice(kind: &Kind) -> i32 {
    match kind {
        Kind::Signal { current } => current.get() + current.get(),
        Kind::Computed { value, dirty } => {
            dirty.set(false);
            value.get()
        }
    }
}

/// Read once after a call that could set it: read there, as Rust reads it.
fn after_a_call(kind: &Kind, bump: impl Fn()) -> i32 {
    match kind {
        Kind::Signal { current } => {
            bump();
            current.get()
        }
        Kind::Computed { value, .. } => value.get() * 2,
    }
}

fn main() {
    let signal = Kind::Signal {
        current: Cell::new(3),
    };
    let computed = Kind::Computed {
        value: Cell::new(2),
        dirty: Cell::new(true),
    };
    println!("{} {}", read(&signal), read(&computed));
    println!("{} {}", read_twice(&signal), read_twice(&computed));
    let bumped = after_a_call(&signal, || {
        if let Kind::Signal { current } = &signal {
            current.set(current.get() + 10);
        }
    });
    println!("{} {}", bumped, after_a_call(&computed, || {}));
}
