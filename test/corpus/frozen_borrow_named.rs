// A `let` of a shared borrow of what holds no cell of its own, a
// `RefCell` field's `Ref` or a `&T`, is read where it's made and keeps
// its name, `const chunks = store.chunks;`, whatever runs while it's
// held: Rust lets nothing change what it's to till its last use (rd658).
// A `&Cell`'s, whose value a call it's held across may set, still reads
// its place where it's used.
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

pub struct Store {
    entries: RefCell<Option<Vec<(String, String)>>>,
    pointer: Cell<usize>,
    chunks: RefCell<Option<HashMap<String, String>>>,
    count: Cell<u32>,
}

fn get_chunk(store: &Store, key: &str) -> Option<String> {
    let chunks = store.chunks.borrow();
    match chunks.as_ref() {
        None => None,
        Some(chunks) => chunks.get(key).cloned(),
    }
}

// Held across a call that sets another of the store's cells, and the
// cursor's.
fn consume(store: &Store, prefix: &str) -> Vec<String> {
    let mut names = Vec::new();
    let entries = store.entries.borrow();
    let Some(entries) = entries.as_ref() else {
        return names;
    };
    let mut i = store.pointer.get();
    while i < entries.len() {
        let entry = &entries[i];
        let Some(name) = entry.0.strip_prefix(prefix) else {
            break;
        };
        bump(store);
        names.push(format!("{name}={}", entry.1));
        i += 1;
    }
    store.pointer.set(i);
    names
}

fn bump(store: &Store) {
    store.count.set(store.count.get() + 1);
}

// A `&Cell`, which the call sets through the store.
fn counted(store: &Store) -> (u32, u32) {
    let count = &store.count;
    let before = count.get();
    bump(store);
    (before, count.get())
}

// The variable the borrow was made through given another store while
// it's held.
fn chunk_count(store: &Store, other: &Store) -> usize {
    let mut from = store;
    let chunks = from.chunks.borrow();
    bump(from);
    from = other;
    bump(from);
    match chunks.as_ref() {
        None => 0,
        Some(chunks) => chunks.len(),
    }
}

fn main() {
    let store = Store {
        entries: RefCell::new(Some(vec![
            ("ab".to_owned(), "1".to_owned()),
            ("ac".to_owned(), "2".to_owned()),
            ("b".to_owned(), "3".to_owned()),
        ])),
        pointer: Cell::new(0),
        chunks: RefCell::new(Some(HashMap::from([("k".to_owned(), "v".to_owned())]))),
        count: Cell::new(0),
    };
    let empty = Store {
        entries: RefCell::new(None),
        pointer: Cell::new(0),
        chunks: RefCell::new(None),
        count: Cell::new(10),
    };
    println!("{:?} {:?}", get_chunk(&store, "k"), get_chunk(&empty, "k"));
    println!("{:?} {:?}", consume(&store, "a"), consume(&empty, "a"));
    println!("{:?} {}", counted(&store), chunk_count(&store, &empty));
    println!("{} {}", store.count.get(), empty.count.get());
}
