// A let chain with an `else` inside an `if` that statements follow is one
// `if` of its tests and its `else`, `if (lanes != null && root != null) {
// .. } else { throw .. }`, as React's attachSuspenseRetryListeners writes
// it inside its `if (enableX && ..)`, not tests that break a labeled
// block; a binding a test reads is read where that test is (ADR 0946,
// amended).
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct Root {
    lanes: Cell<u32>,
}

thread_local! {
    static IN_PROGRESS_LANES: Cell<Option<u32>> = const { Cell::new(None) };
    static IN_PROGRESS_ROOT: RefCell<Option<Rc<Root>>> = const { RefCell::new(None) };
    static ENABLED: Cell<bool> = const { Cell::new(true) };
    static COUNT: Cell<Option<u32>> = const { Cell::new(Some(1)) };
}

fn bump() -> bool {
    COUNT.set(Some(100));
    true
}

// A test after the binding that may change what it reads: the binding
// read where it's bound, before the call.
fn counted(on: bool, log: &mut Vec<u32>) {
    if on {
        if let Some(n) = COUNT.get()
            && bump()
        {
            log.push(n);
        } else {
            log.push(0);
        }
    }
    log.push(9);
}

fn after(log: &mut Vec<u32>, x: u32) {
    log.push(x);
}

fn restore_pending_updaters(root: &Root, lanes: u32) {
    root.lanes.set(lanes);
}

// Its body calls what it will once the tests hold, React's
// restorePendingUpdaters: the chain's tests call nothing still.
fn restore(x: u32, log: &mut Vec<u32>) {
    if x > 1 {
        if ENABLED.get() && x < 10 {
            if let Some(in_progress_lanes) = IN_PROGRESS_LANES.get()
                && let Some(in_progress_root) = IN_PROGRESS_ROOT.with_borrow(Clone::clone)
            {
                restore_pending_updaters(&in_progress_root, in_progress_lanes + x);
            } else {
                panic!("Expected finished root and lanes to be set. This is a bug in React.");
            }
        }
        after(log, x);
    }
}

fn first_of(a: Option<&Vec<u32>>, b: Option<u32>, on: bool, log: &mut Vec<u32>) {
    if on {
        if let Some(items) = a
            && let Some(item) = items.first()
            && let Some(more) = b
        {
            log.push(item + more);
        } else {
            log.push(0);
        }
    }
    log.push(9);
}

// A binding kept, as each read of it needs it: read where its test is.
fn kept(x: u32, a: Option<u32>, en: bool) -> u32 {
    let mut t = 0;
    if x > 1 {
        if en {
            if let Some(l) = a
                && let Some(r) = IN_PROGRESS_ROOT.with_borrow(Clone::clone)
            {
                r.lanes.set(l);
            } else {
                panic!("bad");
            }
        }
        t += 1;
    }
    t
}

fn swap_root() -> bool {
    IN_PROGRESS_ROOT.set(Some(Rc::new(Root { lanes: Cell::new(50) })));
    true
}

// A later test that changes what a binding read: the binding read before it.
fn swapped(en: bool, a: Option<u32>) -> u32 {
    let mut t = 0;
    if a != Some(0) {
        if en {
            if let Some(l) = a
                && let Some(r) = IN_PROGRESS_ROOT.with_borrow(Clone::clone)
                && swap_root()
            {
                t = r.lanes.get() + l;
            } else {
                panic!("none");
            }
        }
        t += 1;
    }
    t + 1
}

struct Holder {
    root: Option<Rc<Root>>,
}

// A binding of a field, read where its test is.
fn held(on: bool, a: Option<u32>, h: &Holder, log: &mut Vec<u32>) {
    if on {
        if let Some(l) = a
            && let Some(r) = h.root.clone()
        {
            r.lanes.set(l);
            log.push(l + r.lanes.get());
        } else {
            log.push(0);
        }
    }
    log.push(9);
}

fn main() {
    let root = Rc::new(Root { lanes: Cell::new(0) });
    let mut log = Vec::new();
    restore(1, &mut log);
    let caught = std::panic::catch_unwind(|| restore(2, &mut Vec::new()));
    println!("{}", caught.is_err());
    IN_PROGRESS_LANES.set(Some(3));
    IN_PROGRESS_ROOT.set(Some(Rc::clone(&root)));
    restore(4, &mut log);
    restore(12, &mut log);
    println!("{} {log:?}", root.lanes.get());
    let items = vec![5, 6];
    let mut seen = Vec::new();
    first_of(Some(&items), Some(1), true, &mut seen);
    first_of(Some(&Vec::new()), Some(1), true, &mut seen);
    first_of(None, Some(1), true, &mut seen);
    first_of(Some(&items), None, false, &mut seen);
    counted(true, &mut seen);
    let holder = Holder { root: Some(Rc::clone(&root)) };
    held(true, Some(2), &holder, &mut seen);
    held(true, None, &holder, &mut seen);
    held(true, Some(2), &Holder { root: None }, &mut seen);
    println!("{seen:?}");
    println!("{} {} {}", kept(2, Some(7), true), kept(2, None, false), root.lanes.get());
    println!("{} {}", swapped(true, Some(1)), swapped(false, Some(1)));
}
