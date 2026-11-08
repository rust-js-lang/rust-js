// A clone of a B-tree of more than eleven keys whose comparisons can be
// seen is std's `clone_subtree`, node by node (#926): as a key's or a
// value's clone panics, what it made is dropped as std's frames drop it,
// and the clone keeps its source's shape, which later searches follow.
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::panic::{AssertUnwindSafe, catch_unwind};

thread_local! {
    static CLONES: Cell<u32> = const { Cell::new(0) };
    static PANIC_AT: Cell<u32> = const { Cell::new(0) };
    static CALLS: Cell<u32> = const { Cell::new(0) };
    static LOG: RefCell<String> = const { RefCell::new(String::new()) };
}

fn log(what: String) {
    LOG.with_borrow_mut(|log| {
        log.push_str(&what);
        log.push(' ');
    });
}

/// Counts a clone, panicking where it's the one `PANIC_AT` names.
fn cloned() {
    CLONES.set(CLONES.get() + 1);
    if CLONES.get() == PANIC_AT.get() {
        panic!("clone {}", CLONES.get());
    }
}

struct Key(u32);

impl Clone for Key {
    fn clone(&self) -> Key {
        cloned();
        Key(self.0 + 1000)
    }
}

impl Drop for Key {
    fn drop(&mut self) {
        log(format!("k{}", self.0));
    }
}

impl PartialEq for Key {
    fn eq(&self, other: &Key) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Key {}

impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Key) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Key {
    fn cmp(&self, other: &Key) -> Ordering {
        CALLS.set(CALLS.get() + 1);
        (self.0 % 1000).cmp(&(other.0 % 1000))
    }
}

struct Val(u32);

impl Clone for Val {
    fn clone(&self) -> Val {
        cloned();
        Val(self.0 + 1000)
    }
}

impl Drop for Val {
    fn drop(&mut self) {
        log(format!("v{}", self.0));
    }
}

fn main() {
    for n in [12, 30, 70] {
        let mut m = BTreeMap::new();
        for i in 0..n {
            m.insert(Key(i * 7 % n), Val(i));
        }
        for at in (1..=2 * n).step_by(3) {
            CLONES.set(0);
            PANIC_AT.set(at);
            let copy = catch_unwind(AssertUnwindSafe(|| m.clone()));
            PANIC_AT.set(0);
            let dropped = LOG.with_borrow_mut(std::mem::take);
            println!("{n} at {at}: {} | {dropped}", copy.is_ok());
            if let Ok(mut copy) = copy {
                CALLS.set(0);
                copy.insert(Key(n / 2), Val(0));
                copy.remove(&Key(n / 3));
                println!("{} calls", CALLS.get());
                drop(copy);
                LOG.with_borrow_mut(|log| log.clear());
            }
        }
        LOG.with_borrow_mut(|log| log.clear());
        let s: BTreeSet<Key> = (0..n).map(|i| Key(i * 5 % n)).collect();
        for at in (1..=n).step_by(4) {
            CLONES.set(0);
            PANIC_AT.set(at);
            let copy = catch_unwind(AssertUnwindSafe(|| s.clone()));
            PANIC_AT.set(0);
            let dropped = LOG.with_borrow_mut(std::mem::take);
            println!("{n} set at {at}: {} | {dropped}", copy.is_ok());
            drop(copy);
            LOG.with_borrow_mut(|log| log.clear());
        }
    }
}
