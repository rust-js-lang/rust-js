// Oracle B's own case of the last shapes the records say (ADR 1204): tuple
// and unit structs, a `Weak` plain and counted, `NonZero`s, std's sets and
// maps, a `NonNull` and a null raw pointer, atomics, and a list deep enough
// that a node's places pass any small depth, each made by a function JS
// calls for its result's shape alone.
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::num::NonZero;
use std::ptr::NonNull;
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicBool, AtomicU32};

pub struct Pair(pub u32, pub String);

pub struct Wide(pub u64);

pub struct Nothing;

#[cfg_attr(rust_js, rust_js::name = "escape-key")]
pub struct Escape;

pub struct Node {
    pub value: u32,
    pub next: Option<Box<Node>>,
}

pub fn pair(n: u32) -> Pair {
    Pair(n, n.to_string())
}

pub fn wide(n: u32) -> Wide {
    Wide(u64::from(n) << 33)
}

pub fn nothing(n: u32) -> Nothing {
    let _ = n;
    Nothing
}

pub fn escape(n: u32) -> Escape {
    let _ = n;
    Escape
}

pub fn weak(n: u32) -> Weak<u32> {
    let strong = Rc::new(n);
    let weak = Rc::downgrade(&strong);
    std::mem::forget(strong);
    weak
}

pub fn alive(n: u32) -> Weak<String> {
    let strong = Rc::new(n.to_string());
    let weak = Rc::downgrade(&strong);
    assert!(weak.upgrade().is_some());
    std::mem::forget(strong);
    weak
}

pub fn small(n: u32) -> NonZero<u32> {
    NonZero::new(n.max(1)).unwrap()
}

pub fn large(n: u32) -> NonZero<u64> {
    NonZero::new(u64::from(n) + (1 << 40)).unwrap()
}

pub fn seen(n: u32) -> HashSet<u32> {
    (0..n.min(4)).collect()
}

pub fn sorted(n: u32) -> BTreeSet<String> {
    (0..n.min(4)).map(|i| format!("s{i}")).collect()
}

pub fn names(n: u32) -> HashMap<u32, String> {
    (0..n.min(4)).map(|i| (i, format!("n{i}"))).collect()
}

pub fn flags(n: u32) -> BTreeMap<u32, bool> {
    (0..n.min(4)).map(|i| (i, i % 2 == 0)).collect()
}

pub fn pointer(n: u32) -> NonNull<u32> {
    NonNull::from(Box::leak(Box::new(n)))
}

pub fn nowhere(n: u32) -> *const u32 {
    let _ = n;
    std::ptr::null()
}

pub fn counter(n: u32) -> AtomicU32 {
    AtomicU32::new(n)
}

pub fn flag(n: u32) -> AtomicBool {
    AtomicBool::new(n % 2 == 0)
}

pub fn list(n: u32) -> Option<Box<Node>> {
    (0..n.min(30)).fold(None, |next, value| Some(Box::new(Node { value, next })))
}

pub fn widened(a: u32, b: u32) -> u64 {
    u64::from(a) + u64::from(b)
}
