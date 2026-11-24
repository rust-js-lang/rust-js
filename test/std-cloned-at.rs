//! Where std raises a `RefCell`'s panic, cloned mutably borrowed by a std
//! type's `clone`, of the types whose clone reaches it by more than one
//! path in std's MIR, so a walk of it can't say which one runs (#2509):
//! each cloned natively, with its `RefCell` at the type parameter it
//! names, and where std's hook is told the panic was raised printed,
//! `<type> <position> <file>:<line>:<column>`. test/std-cloned-at.test.ts
//! runs it and writes what it prints as the oracle,
//! crates/rust-js-lower/src/recognition/compiled_std/cloned_at_oracle.txt.
//! A `BTreeMap` of a few entries, its root a leaf, names its leaf's line,
//! the one rust-js names of any (ADR 0262's exception, the guide's).
//!
//! Nothing here is rust-js's: it's run natively alone, the oracle.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::panic::{self, AssertUnwindSafe};

thread_local! {
    static PLACE: Cell<Option<(&'static str, u32, u32)>> = const { Cell::new(None) };
}

/// Where `clone`, a clone of a value whose `RefCell` is mutably borrowed,
/// is raised, printed as the type's position's.
fn at(name: &str, position: usize, clone: impl FnOnce()) {
    PLACE.set(None);
    let raised = panic::catch_unwind(AssertUnwindSafe(clone));
    assert!(raised.is_err(), "{name}'s clone of a mutably borrowed `RefCell` didn't panic");
    let (file, line, column) = PLACE.get().expect("the hook ran");
    println!("{name} {position} {file}:{line}:{column}");
}

fn main() {
    panic::set_hook(Box::new(|info| {
        let place = info.location().expect("a panic's location");
        PLACE.set(Some((place.file(), place.line(), place.column())));
    }));
    let deque = VecDeque::from([RefCell::new(0u8)]);
    let _held = deque[0].borrow_mut();
    at("alloc::collections::vec_deque::VecDeque", 0, || drop(deque.clone()));
    drop(_held);
    let deque = VecDeque::from([RefCell::new(0u8)]);
    let cell: *const RefCell<u8> = &deque[0];
    let items = deque.into_iter();
    // SAFETY: `into_iter` keeps the deque's buffer where it was, so `cell`
    // is still its item's.
    let _held = unsafe { &*cell }.borrow_mut();
    at("alloc::collections::vec_deque::into_iter::IntoIter", 0, || drop(items.clone()));
    drop(_held);
    let map = BTreeMap::from([(RefCell::new(0u8), 0u8)]);
    let _held = map.keys().next().expect("an entry").borrow_mut();
    at("alloc::collections::btree::map::BTreeMap", 0, || drop(map.clone()));
    drop(_held);
    let map = BTreeMap::from([(0u8, RefCell::new(0u8))]);
    let _held = map[&0].borrow_mut();
    at("alloc::collections::btree::map::BTreeMap", 1, || drop(map.clone()));
    drop(_held);
    let set = BTreeSet::from([RefCell::new(0u8)]);
    let _held = set.first().expect("an item").borrow_mut();
    at("alloc::collections::btree::set::BTreeSet", 0, || drop(set.clone()));
}
