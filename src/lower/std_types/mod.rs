//! std's data structures and values, each lowered in a module of its own
//! (ADR 0314): its JS, its methods, and what it refuses.
//!
//! | Rust | JS | Module |
//! |---|---|---|
//! | `Vec`, `VecDeque`, slices, arrays | `Array` | `vec` |
//! | `HashMap`, `HashSet`, `BTreeMap`, `BTreeSet` | `Map`, `Set`, or `$KeyMap`, `$KeySet` | `map` |
//! | `String`, `&str`, `char` | a string | `text` |
//! | integers, floats | a number, or a `BigInt` | `number` |
//! | `Option` | the value or `undefined`, boxed where it looks like `None` | `option` |
//! | `Result` | `{ TAG, _0 }` | `result` |
//! | `Cell`, `RefCell`, `Mutex`, `RwLock`, atomics | `{ value }` | `cell` |
//! | `Rc`, `Arc` whose counts are read, `Weak` | `{ value, strong, weak }` | `rc` |
//! | `Cow` | `{ TAG, _0 }`, what it borrowed or owns | `cow` |
//! | `OnceCell`, `OnceLock` | `{ value }` of an `Option` | `once` |
//! | `LazyCell`, `LazyLock` | `{ init }`, then `{ value }` | `lazy` |
//! | `BinaryHeap` | an `Array` in heap order | `heap` |
//! | ranges | `{ start, end }` | `range` |
//! | `mpsc` channels | a queue | `channel` |
//! | `Pin` | its pointer | `pin` |
//! | `MaybeUninit`, a `Box` of one | what it holds, or `undefined` | `uninit` |
//! | `TypeId`, `dyn Any` | its type's key, a string; `{ value, impl }` | `any` |
//!
//! Which std type each is, and how much of each is known, is
//! `recognition/registry.rs`'s and `docs/std-coverage.txt`'s.

pub(super) mod any;
pub(super) mod cell;
pub(super) mod channel;
pub(super) mod cow;
pub(super) mod heap;
pub(super) mod lazy;
pub(super) mod map;
pub(super) mod number;
pub(super) mod once;
pub(super) mod option;
pub(super) mod pin;
pub(super) mod range;
pub(super) mod rc;
pub(super) mod result;
pub(super) mod slice;
pub(super) mod text;
pub(super) mod uninit;
pub(super) mod vec;
