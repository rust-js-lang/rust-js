// A variable written more than once, a `match`'s in each arm, then tested
// and stored in a nullable place or returned from a nullable function: as
// it is where nothing writes it between its test and its read, React's
// getIteratorFn's `maybeIterator` and performUnitOfWork's `next`; checked
// where it's written again between, or a closure that captures it by
// `&mut` runs between (#1933).
use std::cell::{Cell, RefCell};

struct Unit {
    id: u32,
}

struct Loop {
    #[cfg_attr(rust_js, rust_js::nullable)]
    work: Cell<Option<&'static Unit>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    last: RefCell<Option<&'static Unit>>,
}

static A: Unit = Unit { id: 1 };
static B: Unit = Unit { id: 2 };

fn begin(n: u32) -> Option<&'static Unit> {
    match n {
        0 => None,
        1 => Some(&A),
        _ => Some(&B),
    }
}

fn perform(work: &Loop, n: u32) {
    let next = match n % 3 {
        0 => begin(n),
        1 => begin(n + 1),
        _ => None,
    };
    if next.is_none() {
        work.last.replace(None);
    } else {
        work.work.set(next);
    }
}

#[cfg_attr(rust_js, rust_js::nullable)]
fn found(n: u32) -> Option<&'static Unit> {
    let found = if n > 4 { begin(n) } else { begin(n % 2) };
    if found.is_some() {
        return found;
    }
    None
}

#[cfg_attr(rust_js, rust_js::nullable)]
fn rewritten(n: u32) -> Option<&'static Unit> {
    let mut found = begin(n);
    if found.is_some() {
        if n > 2 {
            found = begin(0);
        }
        return found;
    }
    None
}

#[cfg_attr(rust_js, rust_js::nullable)]
fn captured(n: u32) -> Option<&'static Unit> {
    let mut found = begin(n);
    let mut clear = || found = None;
    if n > 2 {
        clear();
    }
    if found.is_some() {
        return found;
    }
    None
}

fn shown(unit: Option<&Unit>) -> String {
    unit.map_or("none".to_string(), |unit| unit.id.to_string())
}

fn main() {
    let work = Loop {
        work: Cell::new(None),
        last: RefCell::new(Some(&A)),
    };
    for n in 0..6 {
        perform(&work, n);
        println!("{n}: work {} last {}", shown(work.work.get()), shown(*work.last.borrow()));
        println!("   found {} rewritten {} captured {}", shown(found(n)), shown(rewritten(n)), shown(captured(n)));
    }
}
