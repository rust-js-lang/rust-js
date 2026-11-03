// A thread-local's `with` in another's closure, which reads the outer one's
// parameter, is each one's body where it's called, as one alone is: the
// outer `with`'s closure stayed called where it's made once a closure in
// it read its parameter, until that closure's call was its body too
// (ADR 0840, amended, rd661).
use std::cell::Cell;

struct Internals {
    current: Cell<Option<u32>>,
    calls: Cell<u32>,
}

thread_local! {
    static INTERNALS: Internals = const {
        Internals {
            current: Cell::new(None),
            calls: Cell::new(0),
        }
    };
    static DEFAULT: Cell<u32> = const { Cell::new(7) };
}

fn bump(by: u32) -> u32 {
    println!("bump {by}");
    by
}

fn install() {
    DEFAULT.with(|default| {
        INTERNALS.with(|internals| {
            if internals.current.get() == Some(default.get()) {
                println!("installed already");
            }
            internals.current.set(Some(default.get() + bump(1)));
        });
    });
}

fn counted() -> u32 {
    INTERNALS.with(|internals| {
        DEFAULT.with(|default| internals.calls.set(internals.calls.get() + default.get()));
        internals.calls.get() + bump(2)
    })
}

fn main() {
    install();
    println!("{:?}", INTERNALS.with(|internals| internals.current.get()));
    DEFAULT.with(|default| default.set(8));
    install();
    println!("{:?}", INTERNALS.with(|internals| internals.current.get()));
    println!("{} {}", counted(), counted());
}
