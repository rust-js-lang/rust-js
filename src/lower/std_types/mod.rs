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
//! | ranges | `{ start, end }` | `range` |
//! | `mpsc` channels | a queue | `channel` |
//!
//! Which std type each is, and how much of each is known, is
//! `recognition/registry.rs`'s and `docs/std-coverage.txt`'s.

pub(super) mod cell;
pub(super) mod channel;
pub(super) mod map;
pub(super) mod number;
pub(super) mod option;
pub(super) mod range;
pub(super) mod result;
pub(super) mod text;
pub(super) mod vec;
