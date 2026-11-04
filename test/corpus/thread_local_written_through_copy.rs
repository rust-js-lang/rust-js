// A write through a reference copied out of a thread-local, or a `static`'s,
// is a write of what it points to (#635, ADR 1006, amended): `PTR.get()`
// gives out a `&'static S` whose cell `main` sets, `B` is made of the
// reference `A` holds, and `TO_COUNTER` of one to `COUNTER`, an atomic. So
// what reads that state, `READ`, `READ_A` and `READ_COUNTER`, is made where
// it's first used, as Rust makes it, and reads what `main` wrote.
// An initializer whose value is a field as it is reads only the field:
// `PENDING`, a clone of the `Rc` `REGISTRY` holds, written through after,
// is made as its module loads, as React Flight's
// `TaintRegistryPendingRequests` is.
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};

// Each case's own types, so that no other's may be one place with it.
struct S {
    n: Cell<u32>,
}

struct T {
    n: Cell<u64>,
}

fn leaked() -> &'static S {
    Box::leak(Box::new(S { n: Cell::new(1) }))
}

fn leaked_t() -> &'static T {
    Box::leak(Box::new(T { n: Cell::new(1) }))
}

struct Registry {
    pending: RefCell<Option<Rc<RefCell<Vec<u8>>>>>,
}

fn registry() -> &'static Registry {
    Box::leak(Box::new(Registry {
        pending: RefCell::new(Some(Rc::new(RefCell::new(Vec::new())))),
    }))
}

static COUNTER: AtomicU32 = AtomicU32::new(1);

fn counter() -> &'static AtomicU32 {
    &COUNTER
}

thread_local! {
    static PTR: Cell<Option<&'static S>> = Cell::new(Some(leaked()));
    static READ: u32 = PTR.get().map_or(0, |s| s.n.get());
    static A: &'static T = leaked_t();
    static B: &'static T = A.with(|a| *a);
    static READ_A: u64 = A.with(|a| a.n.get());
    static TO_COUNTER: Cell<Option<&'static AtomicU32>> = Cell::new(Some(counter()));
    static READ_COUNTER: u32 = COUNTER.load(Ordering::Relaxed);
    static REGISTRY: &'static Registry = registry();
    static PENDING: Option<Rc<RefCell<Vec<u8>>>> = REGISTRY.with(|registry| registry.pending.borrow().clone());
}

fn main() {
    if let Some(s) = PTR.get() {
        s.n.set(5);
    }
    println!("{}", READ.with(|r| *r));
    B.with(|b| b.n.set(6));
    println!("{}", READ_A.with(|r| *r));
    if let Some(counter) = TO_COUNTER.get() {
        counter.store(7, Ordering::Relaxed);
    }
    println!("{}", READ_COUNTER.with(|r| *r));
    PENDING.with(|pending| {
        if let Some(pending) = pending {
            pending.borrow_mut().push(3);
        }
    });
    let held = PENDING.with(|pending| pending.as_ref().map(|pending| pending.borrow().len()));
    println!("{held:?}");
}
