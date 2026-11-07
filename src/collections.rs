//! The compiler's maps and sets (ADR 1098): each walked in the order its
//! entries were put in, which the program alone decides, never in a
//! hasher's. A std `HashMap` is walked in an order its process's random
//! keys pick, so a decision made walking one, or the order of what it
//! writes, may differ from one compile of a program to the next: ranges'
//! fixpoint, walking its callees in a `HashSet`'s order, wrote a `u32`'s
//! `n + 1` with its wrap in some compiles and without it in others (#866).
//! The same input gives the same output (ADR 1003); test/architecture.test.ts
//! refuses std's.
//!
//! An entry is taken out by `swap_remove`, which puts the last in its
//! place: an order the program decides too, as each std map's walk was
//! one the code didn't depend on.

/// A map walked in the order its keys were first put in.
pub type IndexMap<K, V> = indexmap::IndexMap<K, V, rustc_hash::FxBuildHasher>;

/// A set walked in the order its items were first put in.
pub type IndexSet<T> = indexmap::IndexSet<T, rustc_hash::FxBuildHasher>;

/// A map each key of which keeps the first value it's given: a session's
/// answers to a question whose answer never changes, each found once, as
/// it's first asked, `std_path`'s (#884). Nothing it holds is changed or
/// taken out, so what reads it reads no state but what it would find.
pub struct OnceMap<K, V>(std::cell::RefCell<IndexMap<K, V>>);

impl<K, V> Default for OnceMap<K, V> {
    fn default() -> Self {
        OnceMap(Default::default())
    }
}

impl<K: std::hash::Hash + Eq, V: Clone> OnceMap<K, V> {
    /// What `key` was given, if it's been given one.
    pub fn get(&self, key: &K) -> Option<V> {
        self.0.borrow().get(key).cloned()
    }

    /// `key`'s value: what it was given first, `value` where it's given
    /// none yet.
    pub fn keep(&self, key: K, value: V) -> V {
        self.0.borrow_mut().entry(key).or_insert(value).clone()
    }
}
