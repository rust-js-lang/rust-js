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
//! | `OnceCell`, `OnceLock` | `{ value }` of an `Option`; in a field, the property | `once` |
//! | `LazyCell`, `LazyLock` | `{ init }`, then `{ value }` | `lazy` |
//! | `BinaryHeap` | an `Array` in heap order | `heap` |
//! | ranges | `{ start, end }` | `range` |
//! | `mpsc` channels | a queue | `channel` |
//! | `Pin` | its pointer | `pin` |
//! | `MaybeUninit`, a `Box` of one | what it holds, or `undefined` | `uninit` |
//! | `catch_unwind`, `resume_unwind` | a `try`, `$catchUnwind(f)`; `throw $unwinding(payload)` | `unwind` |
//! | `TypeId`, `dyn Any` | its type's key, a string; `{ value, impl }` | `any` |
//! | `env::args`, `process::exit`: Node's process | its helper, `$args()` | `process` |
//! | `Path`, `PathBuf` | its text; its methods `path.js`'s | `path` |
//! | `panic::Location`, `#[track_caller]` | `{ file, line, column }` | `location` |
//! | the allocator, `Layout` | an array of slots; `{ size, align }` | `alloc` |
//! | `SystemTime`, `Instant` | its nanoseconds, a `BigInt` | `clock` |
//! | `CStr`, `CString` | its bytes with its nul, an `Array` | `c_str` |
//! | `io::Error` | a `$IoError`, its kind and what it holds | `io_error` |
//! | `io::Cursor`; `&[u8]` and it as readers | `{ inner, pos }`; `$sliceReader`, `$cursorReader(TAsRef)` | `io` |
//! | `DefaultHasher`, `SipHasher`, std's `Hash` | a `$SipHasher`; the writes std's impls make | `hash` |
//!
//! Which std type each is, and how much of each is known, is
//! `recognition/registry.rs`'s and `docs/std-coverage.txt`'s.

pub(super) mod alloc;
pub(super) mod any;
pub(super) mod backtrace;
mod btree;
pub(in crate::lower) mod c_str;
pub(super) mod cell;
pub(super) mod channel;
pub(super) mod clock;
pub(super) mod cow;
pub(super) mod future;
pub(super) mod hash;
pub(super) mod heap;
pub(super) mod io;
pub(super) mod io_error;
pub(super) mod lazy;
pub(super) mod location;
pub(super) mod map;
mod map_keys;
pub(super) mod number;
pub(super) mod once;
pub(super) mod option;
pub(super) mod path;
pub(super) mod pin;
pub(super) mod process;
pub(super) mod range;
pub(super) mod rc;
pub(super) mod result;
pub(super) mod slice;
pub(super) mod text;
pub(super) mod uninit;
pub(super) mod unwind;
mod values;
pub(super) mod vec;
