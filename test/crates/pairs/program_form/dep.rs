//! A library's generic functions as React's `createContext` and
//! `useState` are, given what its own crate's calls give them, nothing: its
//! JS is the program's, which JS calls as React's own JS does. A consumer
//! that gives them a value to copy or drop compiles its own (ADR 0914), and
//! the impls it uses that the library doesn't, and the functions that copy
//! what it changes in place where the library changes none.
use std::cell::Cell;
use std::fmt::{self, Debug, Write};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::rc::Rc;

pub struct Context<T: 'static> {
    pub current_value: Cell<T>,
    pub current_value2: Cell<T>,
}

pub fn create_context<T: Copy>(default_value: T) -> Rc<Context<T>> {
    Rc::new(Context {
        current_value: Cell::new(default_value),
        current_value2: Cell::new(default_value),
    })
}

/// The dispatcher a hook is called in, as React's `resolveDispatcher`: none
/// outside of a render, which panics.
fn resolve_dispatcher(dispatcher: Option<&'static str>) -> &'static str {
    match dispatcher {
        Some(dispatcher) => dispatcher,
        None => panic!("Invalid hook call."),
    }
}

pub fn use_state<S>(initial_state: S, dispatcher: Option<&'static str>) -> (S, &'static str) {
    let dispatcher = resolve_dispatcher(dispatcher);
    (initial_state, dispatcher)
}

/// Asks its dispatcher before it drops what it's given: a panic drops it
/// then, by the flag optimized MIR keeps of whether it's still there.
pub fn dispatched<T>(value: T, dispatcher: Option<&'static str>) -> usize {
    let length = resolve_dispatcher(dispatcher).len();
    drop(value);
    length
}

/// Two copies of `value`, its own copy here, and `dropped` dropped.
pub fn pair_of<T: Copy, D>(value: T, dropped: D) -> (T, T) {
    drop(dropped);
    (value, value)
}

/// An hour of the day, whose `Display` fails past one (ADR 0187).
pub struct Hour(pub u32);

impl fmt::Display for Hour {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0 > 23 {
            return Err(fmt::Error);
        }
        write!(f, "{:02}h", self.0)
    }
}

/// Writes `hour`, failing where its `Display` does, and drops what it's
/// given.
pub fn written<T>(value: T, hour: &Hour) -> fmt::Result {
    let mut text = String::new();
    let result = write!(text, "{hour}");
    drop(value);
    result
}

/// The size of what it's given, which it drops (ADR 0145).
pub fn footprint<T>(value: T) -> usize {
    let size = std::mem::size_of::<T>();
    drop(value);
    size
}

/// Shown only by its consumer, by its derived `Debug`.
#[derive(Debug)]
pub struct Label(pub &'static str);

/// Two of what it's given, which its own crate gives a struct it changes:
/// it takes a copy, as its contract says, and its consumer gives it one.
pub fn twice<T: Copy>(value: T) -> (T, T) {
    (value, value)
}

#[derive(Clone, Copy)]
struct Mark {
    n: i32,
}

pub fn marks() -> i32 {
    let (mut a, b) = twice(Mark { n: 1 });
    a.n += 10;
    a.n + b.n
}

/// Each is itself, as React's `Fiber` is, by its hand-written `PartialEq`
/// and `Hash`, which the library never asks itself: its consumer keys a
/// `HashSet` by it, which is a JS `Set` of it, and hashes it.
pub struct Key {
    pub label: &'static str,
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for Key {}

impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(self, state);
    }
}

/// Equal by its last digit, as its hand-written `PartialEq` and `Hash`
/// say, which only the consumer gives generic code.
pub struct Digit(pub u32);

impl PartialEq for Digit {
    fn eq(&self, other: &Self) -> bool {
        self.0 % 10 == other.0 % 10
    }
}

impl Hash for Digit {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.0 % 10).hash(state);
    }
}

/// Twice its value, as heck's `to_kebab_case` is its trait's: a consumer
/// calls it through the dictionary the library exports.
pub trait Doubled {
    fn doubled(&self) -> u32;
}

impl Doubled for Digit {
    fn doubled(&self) -> u32 {
        self.0 * 2
    }
}

/// Derived, a key a `HashSet` holds by its value.
#[derive(PartialEq, Eq, Hash, Debug)]
pub struct Tag {
    pub id: u32,
}

/// Whether `a` and `b` are equal, by their `PartialEq`.
pub fn same<T: PartialEq>(a: &T, b: &T) -> bool {
    a == b
}

/// What std's hasher makes of `value`, by its `Hash`.
pub fn hash_of<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

/// What `value` shows as, by its `Debug`.
pub fn shown<T: Debug>(value: &T) -> String {
    format!("{value:?}")
}

/// A `Copy` struct the library never changes, which its consumer does.
#[derive(Clone, Copy, Debug)]
pub struct Spot {
    pub x: i32,
}

/// The first of `spots`, a copy of it.
pub fn first_spot(spots: &[Spot]) -> Spot {
    spots[0]
}

/// The first of `spots`, `n` calls down: of `inner`, which calls it back,
/// or, at the bottom, `first_spot`'s copy.
pub fn outer(spots: &[Spot], n: usize) -> Spot {
    if n > 0 { inner(spots, n - 1) } else { first_spot(spots) }
}

pub fn inner(spots: &[Spot], n: usize) -> Spot {
    outer(spots, n)
}

/// The first of `pairs`, a copy of it.
pub fn first_pair(pairs: &[(i32, i32)]) -> (i32, i32) {
    pairs[0]
}

/// What comes first of what holds `Spot`s, a copy of it.
pub trait First {
    fn first(&self) -> Spot;
}

/// `Spot`s a `Vec` holds.
pub struct Spots(pub Vec<Spot>);

impl First for Spots {
    fn first(&self) -> Spot {
        self.0[0]
    }
}

/// The first of `spots`, by its `First`.
pub fn first_of(spots: &Spots) -> Spot {
    spots.first()
}
