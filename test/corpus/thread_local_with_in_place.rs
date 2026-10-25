// A thread-local's `with(|x| ..)` is its closure's body where it's called,
// its parameter the thread-local itself: as a statement, as an `if`'s test
// and as a value, each run where the call was, before what's after it; one
// that returns early stays a closure, whose `return` is its own (ADR 0840).
// A shadow of its parameter, `let held = held.as_ref().unwrap();`, is a
// name of its own from there on, `const held$1 = $unwrap(held);`, the
// thread-local never set (ADR 0840, amended).
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct Internals {
    calls: Cell<u32>,
    flag: Cell<Option<bool>>,
}

thread_local! {
    static INTERNALS: Internals = const {
        Internals {
            calls: Cell::new(0),
            flag: Cell::new(None),
        }
    };
    static HELD: Option<Rc<RefCell<Vec<u32>>>> = Some(Rc::new(RefCell::new(vec![1])));
    static ITEMS: RefCell<Vec<u32>> = RefCell::new(vec![1, 2, 3]);
}

fn bump(by: u32) -> u32 {
    println!("bump {by}");
    by
}

fn seen() -> bool {
    if INTERNALS.with(|internals| {
        let before = internals.calls.get();
        internals.calls.set(before + bump(1));
        before > 0
    }) {
        return true;
    }
    false
}

fn flagged() -> bool {
    INTERNALS.with(|internals| internals.flag.get().is_some_and(|flag| flag))
}

fn early(limit: u32) -> u32 {
    let found = INTERNALS.with(|internals| {
        if internals.calls.get() > limit {
            return internals.calls.get();
        }
        internals.calls.set(internals.calls.get() + bump(2));
        0
    });
    found + 1
}

fn skipped(skip: bool) {
    INTERNALS.with(|internals| {
        if skip {
            return;
        }
        internals.calls.set(bump(5));
    });
    println!("after {}", INTERNALS.with(|internals| internals.calls.get()));
}

fn push_held(value: u32) {
    HELD.with(|held| {
        let held = held.as_ref().unwrap();
        held.borrow_mut().push(value);
    });
}

fn counted() -> usize {
    ITEMS.with_borrow(|items| {
        let items = items.iter().filter(|&&n| n > 1).count();
        items + 1
    })
}

fn summed() -> u32 {
    ITEMS.with_borrow(|items| {
        let mut items = items.iter().sum::<u32>();
        items += bump(4);
        items
    })
}

fn main() {
    INTERNALS.with(|internals| {
        internals.flag.set(Some(false));
        internals.calls.set(bump(0));
    });
    println!("{} {}", seen(), seen());
    println!("{}", flagged());
    INTERNALS.with(|internals| internals.flag.set(Some(true)));
    println!("{} {} {}", flagged(), early(1), early(1));
    let total = INTERNALS.with(|internals| {
        let calls = internals.calls.get();
        calls * 10
    }) + bump(3);
    println!("{total}");
    skipped(true);
    skipped(false);
    push_held(2);
    push_held(3);
    println!("{:?}", HELD.with(|held| held.as_ref().unwrap().borrow().clone()));
    println!("{} {}", counted(), summed());
    println!("{:?}", ITEMS.with_borrow(|items| items.clone()));
}
