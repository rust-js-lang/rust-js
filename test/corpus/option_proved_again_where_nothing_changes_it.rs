// What a test proved of an `Option`, read again where nothing between can
// change it, MIR's to say (#1933's removal of tidy's presence rewrites):
// two `get()`s of a thread-local's `Cell`, React's
// resolveCurrentlyRenderingComponent (rd443), one with a call between that
// sets it to `None`, which panics as Rust's does; a local lent `&mut` to
// `get_or_insert_with` before its test, React's Flight client's `deferred`;
// and an `Option` a closure captures by reference, tested and read in it.
use std::cell::{Cell, RefCell};
use std::panic::catch_unwind;

thread_local! {
    static CURRENT: Cell<Option<&'static str>> = const { Cell::new(None) };
}

fn resolve() -> &'static str {
    if CURRENT.get().is_none() {
        panic!("Invalid hook call.");
    }
    CURRENT.get().unwrap()
}

fn resolve_after(run: impl Fn()) -> &'static str {
    if CURRENT.get().is_none() {
        panic!("Invalid hook call.");
    }
    run();
    CURRENT.get().unwrap()
}

struct Chunk {
    value: RefCell<Option<Vec<u32>>>,
    blocked: bool,
}

fn defer(items: Vec<u32>, chunk: &Chunk) {
    let mut deferred: Option<Vec<u32>> = None;
    for item in items {
        deferred.get_or_insert_with(Vec::new).push(item);
    }
    if deferred.is_some() && chunk.blocked {
        *chunk.value.borrow_mut() = deferred;
    }
}

fn first_or(value: Option<&'static str>, fallback: &'static str, run: &dyn Fn(&dyn Fn() -> bool) -> bool) -> bool {
    run(&|| {
        if value.is_some() {
            return value.unwrap_or(fallback) == fallback;
        }
        false
    })
}

fn after_none(value: Option<&'static str>, fallback: &'static str, run: &dyn Fn(&dyn Fn() -> bool) -> bool) -> bool {
    run(&|| {
        if value.is_none() {
            return false;
        }
        let kept = value.unwrap_or(fallback);
        kept == fallback
    })
}

fn main() {
    println!("{}", catch_unwind(resolve).is_err());
    CURRENT.set(Some("component"));
    println!("{}", resolve());
    println!("{}", resolve_after(|| {}));
    println!("{}", catch_unwind(|| resolve_after(|| CURRENT.set(None))).is_err());
    let chunk = Chunk { value: RefCell::new(None), blocked: true };
    defer(vec![4, 5], &chunk);
    println!("{:?}", chunk.value.borrow());
    defer(vec![], &chunk);
    println!("{:?}", chunk.value.borrow());
    let run = |f: &dyn Fn() -> bool| f();
    println!("{} {} {}", first_or(Some("a"), "b", &run), first_or(Some("b"), "b", &run), first_or(None, "b", &run));
    println!("{} {} {}", after_none(Some("a"), "b", &run), after_none(Some("b"), "b", &run), after_none(None, "b", &run));
}
