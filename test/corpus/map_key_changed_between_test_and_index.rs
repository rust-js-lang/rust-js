//@ library-differs
// A map's `m[k]` after `m.contains_key(k)` can't panic only while what
// the lookup reads of `k` stays (#1844, rd558). A call between that changes
// it, through a `Cell` a shared reference reaches or a static a
// hand-written `Hash` reads, and Rust's `m[k]` panics, so the index is
// checked: a `Cell` in a struct whose hand-written `Hash` reads it, and a
// key whose `Hash` and `Eq` read a static atomic. A `String` key, whose
// lookup reads the same until it's written, is the map's `get`; a key of a
// type parameter's, whose lookup may be any, is checked. (An
// `Rc<Cell<i32>>` or a `&Cell<i32>` keying a `BTreeMap` by std's own `Ord`
// isn't held either, but rust-js refuses comparing `Cell`s for now.)
use std::any::Any;
use std::cell::Cell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicU32, Ordering};

fn message(payload: Box<dyn Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "another payload".to_string()
    }
}

fn check<T: std::fmt::Debug>(what: &str, f: impl FnOnce() -> T) {
    match panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(value) => println!("{what}: {value:?}"),
        Err(payload) => println!("{what}: {}", message(payload)),
    }
}

/// A key whose hash and equality read its `Cell`.
struct Counted(Cell<u32>);

impl PartialEq for Counted {
    fn eq(&self, other: &Self) -> bool {
        self.0.get() == other.0.get()
    }
}

impl Eq for Counted {}

impl Hash for Counted {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.get().hash(state);
    }
}

static SALT: AtomicU32 = AtomicU32::new(0);

/// A key whose hash and equality read a static.
struct Salted(u32);

impl PartialEq for Salted {
    fn eq(&self, other: &Self) -> bool {
        self.0 + SALT.load(Ordering::Relaxed) == other.0
    }
}

impl Eq for Salted {}

impl Hash for Salted {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.0 + SALT.load(Ordering::Relaxed)).hash(state);
    }
}

fn counted(m: &HashMap<Counted, u32>, k: &Counted, bump: &dyn Fn()) -> u32 {
    if m.contains_key(k) {
        bump();
        return m[k];
    }
    0
}

fn salted(m: &HashMap<Salted, u32>, k: &Salted, bump: &dyn Fn()) -> u32 {
    if m.contains_key(k) {
        bump();
        return m[k];
    }
    0
}

/// Of a key of any type: what its lookup reads, unknown.
fn generic<K: Hash + Eq>(m: &HashMap<K, u32>, k: &K, bump: &dyn Fn()) -> u32 {
    if m.contains_key(k) {
        bump();
        return m[k];
    }
    0
}

fn named(m: &HashMap<String, u32>, k: &String, touch: &dyn Fn()) -> u32 {
    if m.contains_key(k) {
        touch();
        return m[k];
    }
    0
}

fn main() {
    let key = Counted(Cell::new(0));
    let m = HashMap::from([(Counted(Cell::new(0)), 7)]);
    check("counted", || counted(&m, &key, &|| key.0.set(1)));
    check("counted unchanged", || counted(&m, &Counted(Cell::new(0)), &|| {}));

    let m = HashMap::from([(Salted(5), 10)]);
    check("salted", || salted(&m, &Salted(5), &|| SALT.store(1, Ordering::Relaxed)));

    let key = Counted(Cell::new(0));
    let m = HashMap::from([(Counted(Cell::new(0)), 12)]);
    check("generic", || generic(&m, &key, &|| key.0.set(1)));
    check("generic of a string", || generic(&HashMap::from([(String::from("b"), 13)]), &String::from("b"), &|| {}));

    let m = HashMap::from([(String::from("a"), 11)]);
    check("named", || named(&m, &String::from("a"), &|| println!("touched")));
}
