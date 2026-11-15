// A scope guard that restores a thread-local `RefCell` no borrow of which
// is checked, alien-signals' `Finally(|| ACTIVE_SUB.set(prev_sub.clone()))`,
// drops in a `finally`: its `set` can't find the cell borrowed, the old
// `Rc` it drops runs nothing, and an `Rc`'s clone never unwinds (#1930).
// One whose cell a held borrow is checked of, one whose old value has a
// destructor, and one whose closure panics keep their abort-safe cleanup.
use std::cell::RefCell;
use std::panic;
use std::rc::Rc;

struct Finally<F: FnMut()>(F);

impl<F: FnMut()> Drop for Finally<F> {
    fn drop(&mut self) {
        (self.0)();
    }
}

struct Node {
    name: &'static str,
}

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

thread_local! {
    static ACTIVE: RefCell<Option<Rc<Node>>> = const { RefCell::new(None) };
    static HELD: RefCell<Option<Rc<Node>>> = const { RefCell::new(None) };
    static LOUD: RefCell<Option<Loud>> = const { RefCell::new(None) };
}

fn active() -> &'static str {
    ACTIVE.with_borrow(|a| a.as_ref().map_or("none", |n| n.name))
}

fn with_active(node: Rc<Node>, work: impl Fn()) {
    let prev = ACTIVE.replace(Some(node));
    let _restore = Finally(|| ACTIVE.set(prev.clone()));
    work();
}

fn with_read(work: impl Fn()) {
    let prev = ACTIVE.with(|a| a.borrow().clone());
    let _restore = Finally(|| ACTIVE.set(prev.clone()));
    work();
}

fn with_held(node: Rc<Node>, work: impl Fn()) {
    let prev = HELD.replace(Some(node));
    let _restore = Finally(|| HELD.set(prev.clone()));
    work();
}

fn with_loud(name: &'static str, work: impl Fn()) {
    let _restore = Finally(|| LOUD.set(Some(Loud(name))));
    work();
}

fn checked(limit: usize, work: impl Fn()) {
    let _restore = Finally(|| {
        if ACTIVE.with_borrow(|a| a.as_ref().map_or(0, |n| n.name.len())) > limit {
            panic!("too long");
        }
    });
    work();
}

fn main() {
    with_active(Rc::new(Node { name: "outer" }), || {
        println!("in {}", active());
        with_active(Rc::new(Node { name: "inner" }), || println!("in {}", active()));
        println!("back to {}", active());
    });
    println!("after {}", active());
    let thrown = panic::catch_unwind(|| {
        with_active(Rc::new(Node { name: "thrown" }), || panic!("boom"));
    });
    println!("caught {} at {}", thrown.is_err(), active());
    with_active(Rc::new(Node { name: "read" }), || {
        let read = panic::catch_unwind(|| {
            with_read(|| {
                ACTIVE.set(None);
                panic!("cleared");
            })
        });
        println!("read {} at {}", read.is_err(), active());
    });
    // A borrow held while the guard's `set` runs: its cell's borrows are
    // checked, and the `set` panics, as it finds the cell borrowed.
    let borrowed = panic::catch_unwind(|| {
        HELD.with_borrow(|held| {
            println!("held {}", held.is_some());
            with_held(Rc::new(Node { name: "held" }), || println!("holding"));
        });
    });
    println!("borrowed {}", borrowed.is_err());
    with_loud("first", || println!("loud"));
    with_loud("second", || println!("louder"));
    checked(3, || println!("checked"));
    let refused = panic::catch_unwind(|| {
        with_active(Rc::new(Node { name: "long" }), || checked(3, || println!("long")));
    });
    println!("refused {} at {}", refused.is_err(), active());
    // Dropped here, where JS drops it: a thread-local's at exit is std's
    // best effort.
    drop(LOUD.take());
}
