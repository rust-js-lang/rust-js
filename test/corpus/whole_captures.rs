// A number a closure counts in, taken by value, is whole, whose JS text is
// Rust's (rd653, ADR 0994, amended): react-markup's `let stash_error_idx =
// Cell::new(1.0)` that its `move |error| ..` reads and sets one more, `''
// + stashErrorIdx++` upstream. One set before the closure takes it, `-0.0`;
// lent to a function, to `get_mut` or to a closure inside; past 2^53;
// stepped by a half; an `f64` written by `+=`; one taken by reference; and
// one a constant's closure holds keep Rust's text of any `f64`.
use std::cell::Cell;

fn counted(f: &dyn Fn() -> String) -> String {
    [f(), f(), f()].join(",")
}

fn set_negative_zero(cell: &Cell<f64>) {
    cell.set(-0.0);
}

fn ids() -> String {
    let stash_error_idx = Cell::new(1.0);
    let handle = move || {
        let id = stash_error_idx.get().to_string();
        stash_error_idx.set(stash_error_idx.get() + 1.0);
        id
    };
    let down = Cell::new(0.0);
    let settle = move || {
        down.set(down.get() - 1.0);
        format!("{}", down.get())
    };
    format!("{} {}", counted(&handle), counted(&settle))
}

fn kept() -> Vec<String> {
    let set_first = Cell::new(1.0);
    set_first.set(-0.0);
    let set_first = move || set_first.get().to_string();

    let lent = Cell::new(1.0);
    let lent = move || {
        set_negative_zero(&lent);
        lent.get().to_string()
    };

    let mut mutable = Cell::new(1.0);
    let mut mutable = move || {
        *mutable.get_mut() = -0.0;
        mutable.get().to_string()
    };

    let inner = Cell::new(1.0);
    let inner = move || {
        let zero = || inner.set(-0.0);
        zero();
        inner.get().to_string()
    };

    let big = Cell::new(9007199254740992.0 * 2.0);
    let big = move || {
        big.set(big.get() * 1e6);
        big.get().to_string()
    };

    let halves = Cell::new(1.0);
    let halves = move || {
        halves.set(halves.get() + 0.5);
        halves.get().to_string()
    };

    let mut added = 1.0;
    let mut added = move || {
        added -= 1.0;
        added *= -1.0;
        added.to_string()
    };

    let borrowed = Cell::new(1.0);
    let by_reference = || {
        borrowed.set(-0.0);
        borrowed.get().to_string()
    };

    vec![
        set_first(),
        lent(),
        mutable(),
        inner(),
        big(),
        halves(),
        added(),
        by_reference(),
    ]
}

fn main() {
    println!("{}", ids());
    println!("{:?}", kept());
}
