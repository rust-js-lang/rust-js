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
