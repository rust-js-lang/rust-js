# Crates from crates.io, compiled to JS

What a shared model's dependencies come to under rust-js (ROADMAP M8.2).
`bun scripts/crate-corpus.ts` compiles each crate below in a Cargo project
of its own, every library of its graph by rust-js as Cargo's
`RUSTC_WRAPPER`, for its target, and records what each refused first, or
what Cargo said where a crate failed with no refusal: a crash.

Compiled isn't run. Where a crate compiles, its probe,
[`scripts/crate-corpus/<name>.rs`](../scripts/crate-corpus/), a library that
uses it as a shared model would, is compiled with it, and its `report()` run,
the JS's beside native Rust's: **runs** where they're the same. A derive's
code is the probe's: strum's and thiserror's compiled before what they write
in a crate that uses them did (ADR 0186).

Each night, `bun scripts/crate-corpus.ts --check` measures every crate
again and fails where one does worse than its row's verdict, the bar.
A verdict of `runs (refused: #467)` is a crate an open issue says does worse
now: it holds while the crate does just that, and fails once it does better,
when the fix writes the verdict again.

Measured 2026-10-19, after ADR 0462, Rust 1.99.0. This table is the
corpus: each row's crate, at exactly the release its first cell names,
built with the features in its parentheses alone, its default ones off,
where it names any. Each release is the newest on 2026-11-02. A crate
moves to a new release only by an edit of its row, measured as it's made,
and `bun scripts/crate-corpus.ts` refuses a row of no exact release (ADR
1067).

Every crate of each row's graph is pinned too: its lock,
[`scripts/crate-corpus/<name>.lock`](../scripts/crate-corpus/), a
Cargo.lock that Cargo is held to (`--locked`), so a dependency's new
release can't change a verdict either. Each was resolved on 2026-11-03.
`bun scripts/crate-corpus.ts --lock <name> ..` resolves a crate's graph
again, at the newest releases its requirements allow, writes its lock and
measures it. That's how a graph moves, on purpose, and how a row whose
release or features were edited gets a lock that matches it. A lock that
doesn't match its row is refused, with the command that rewrites it. A crate
of a graph that rust-js crashed on, rather than refused, is printed as
crashed (#511).

| Crate | Verdict | First refusals in its graph |
|---|---|---|
| strum 0.27.2 (`std`, `derive`) | runs | |
| either 1.19.0 | runs | measured 2026-11-02, at 1.19.0, after ADR 0687's amendment (#475): its own `rfold` and `nth_back` given its sides' dictionaries |
| itertools 0.14.0 | compiles | measured 2026-11-02, after ADRs 0202, 0687 and 0545 amended (#473): its probe is refused at a closure that holds part of a value with a destructor; on 2026-10-22, after ADR 0552's amendment, at `collect_vec()` of a `chunk_by` group's `copied()`, a `Group` whose `Drop` the library's `collect_vec` takes none of |
| indexmap 2.14.2 | runs | a `Map`, rustc builds it (ADR 0424) |
| bitflags 2.13.2 | runs | |
| smallvec 1.16.4 | runs | measured 2026-11-02, at 1.16.4, after ADRs 0145, 0099 and 0914 amended (#476): `size_of::<A::Item>()` of a library's dictionary, a `&mut usize` of a tuple given on, `.map(Self)` |
| arrayvec 0.7.8 | runs | measured 2026-11-02, after #468's fix |
| hashbrown 0.17.1 | runs | its maps std's, rustc builds it (ADR 0423) |
| thiserror 2.0.21 | runs | |
| anyhow 1.0.104 | refused | measured 2026-11-03, at its lock, after ADR 0454's amendment (#516): `{}` of a `dyn Error + Send + Sync` made where it's shown; its `Mut<'a, T>`, a `PhantomData<&'a mut T>` beside its pointer, compiles; its pointer casts (ADR 0562), `Arguments::as_str` (ADR 0034), `Backtrace` (ADR 0568) and `Display::fmt` on a held `Formatter` (ADR 0461) compile |
| once_cell 1.21.4 | refused | `std::thread::current` called |
| semver 1.0.28 | compiles | measured 2026-10-21, after ADR 0562: its tagged pointers, inline identifiers and pointer constants compile; no probe runs it yet |
| uuid 1.29.0 | runs | measured 2026-10-21, after ADR 0555 |
| url 2.5.8 | blocked | measured 2026-11-03, at its lock: 13 of its 16 crates compile. yoke: raw pointers to erase lifetimes. litemap 0.8.3, at `Hash::hash` in its derived `Hash`, since 88383e402, litemap 0.8.1 the same (#514). utf_types 1.0.0, new to the graph through utf8_iter 1.1.0, at `core::ptr::from_ref` |
| regex 1.13.1 | blocked | aho-corasick: memchr's `memmem::Finder`, which ADR 0529 doesn't ship; its unaligned reads compile (ADR 0562). regex-syntax 0.8.11, measured 2026-11-03: refused at a `&mut ast::parse::Parser` in a generic function's parameters. Until #511, rust-js crashed on it, a stack overflow (regex-syntax 0.8.8 at e18f3d9d1 too), that the script counted as neither compiled nor refused |
| rust_decimal 1.43.0 | runs | measured 2026-11-02, after ADR 0176's and 0914's amendments (#468): arrayvec's `ArrayVecImpl` impl it compiles has its `CAPACITY = CAP`, and its `a * b` is called through `decimalMul` |
| chrono 0.4.45 (`alloc`) | runs | measured 2026-11-02, after ADR 0592's and 0051's amendments (#467): its `date` module's dictionaries, in a cycle with `time_delta`'s constants, and `format::parse`'s call of `parse_internal` |
| time 0.3.55 (`alloc`) | blocked | powerfmt: a `Formatter` of its own; its `MaybeUninit` buffer's pointers compile (ADR 0562); deranged compiles (ADR 0165) |
| heck 0.5.0 | runs | a callable that writes is a writer (ADR 0460) |
| base64 0.22.1 | runs | measured 2026-11-02, after ADR 0914's amendment (#474); an `io::Error` a value, its writer's errors thrown (ADR 0462) |
| hex 0.4.3 | runs | measured 2026-10-21 |
| num-traits 0.2.19 | compiles | a float's `log`, `sin_cos`, `rem_euclid` and the like (ADR 0064) |

On 2026-11-03, at each crate's lock, 17 of 22 compile. 15 have a probe:
14 of those run as natively, and itertools' probe is refused (its row).
uuid, measured 2026-10-18, after ADRs 0447 to 0450, compiles: its clock,
`SystemTime::now()`; its `transmute`s of a `&[u8; 16]` as a `&Uuid` and a
`&Uuid` as a `&Hyphenated`; its lock's `PoisonError::into_inner`; and
`Deref` of a `MutexGuard<C>` in its generic clock sequence.
hashbrown and indexmap, measured 2026-10-16, run: rustc builds them, and
their maps are a `Map` and a `Set` (ADRs 0423, 0424).
smallvec, measured 2026-10-16, after ADRs 0417 to 0422, runs whole: its
`SmallVec` inline and spilled to the heap, from a `Vec` and back, and what
it drops, and when, of items with a destructor: a `Drain` taken in part,
`truncate`'s, `resize`'s value.
either, measured 2026-10-16, after ADR 0425, compiles past its `Future`,
`io::Read`, `BufRead` and `Seek` impls, now allowed and never lowered, and
stops at generic code that calls `Extend::extend` on an `L: Extend<A>`;
itertools stopped there too.
itertools, measured 2026-10-21, after ADRs 0532 to 0552, runs whole: its
probe's 36 lines of adapters and consumers, `pad_using` from its back, `tuples`,
`collect_tuple`, `multi_cartesian_product`, `chunk_by`, `tee` and the rest,
as native Rust gives them.
arrayvec, measured 2026-10-16, after ADRs 0407 and 0392's amendments, runs
whole: its `ArrayVec` and `ArrayString`, of raw pointers into an array of
`MaybeUninit`s, and its `io::Write`. Earlier, 4 of 16 compiled, and each of
the 3 with a probe ran as natively, as on 2026-10-05. Two had gone back with the lowering from MIR (ADR 0364), and
are fixed: a `&mut` of what a generic `Option`'s `Some` holds, chrono's
(ADR 0051), and another crate's generic writer given a `Formatter`,
bitflags' `to_writer` (ADR 0180); and chrono's `enum Void {}` was declared
`type Void = ;`, now `never` (ADR 0196). What stops the most, by the crates
it stops:

1. **Raw memory,** a pointer tested for null, read as another type, into
   what JS may hold a copy of, `copy_nonoverlapping`: 7, though a raw
   pointer to a place is one now (ADR 0407).
2. **`handle_alloc_error`**, and `Extend::extend` of an `L: Extend<A>`: 2 each.
3. **One each:** a `Formatter` handed to a closure, semver's,
   `thread::current`,
   `assert_unchecked`, `size_of` of a type parameter, `Ref`'s `Deref`,
   `{:?}` of a `PhantomData`, `rev` of a lazy iterator,
   `from_utf8_lossy`.

The measuring itself had gone wrong: since ADR 0412 every crate Cargo
builds through rust-js is given `--declarations`, and the script, which
made Cargo's environment its own way, had no `RUST_JS_TYPESCRIPT`, so
TypeScript's printer wasn't found, and three crates stopped there. It's
`cargoEnv`'s now, the build's own.

## Fixed by measuring

- **An or-pattern's moves, an arm's temporaries and `Box::from` of a
  `&str`:** chrono's last 4, and it compiles (ADR 0191).
- **A loop over, or a fold of, a generic iterator:** taken as given no
  destructor, and a call that gives one is an error, at the call: chrono's
  9 refusals are 4, and litemap's loop compiles (ADR 0190).
- **std's `Duration`, `panic!("{}", x)`, a slice into an array and
  `into_iter()` of the crate's iterator:** chrono's 18 refusals are 9 (ADRs
  0188, 0189).
- **A writer's `Err(fmt::Error)`,** chrono's, at four places: thrown, with
  what was written before it, each consumer taking it as std's does (ADR
  0187).
- **What another crate's derive writes,** strum's `IntoEnumIterator` and
  thiserror's `From`, was skipped as std's derives' is; `Display` of a
  `fmt::Arguments`; std's `Error` of a parse error as a dictionary: strum
  and thiserror run (ADR 0186).
- **`ok_or_else` of a value with a destructor, a temporary of one in a
  branch, and `by_ref()` of an iterator of the crate's:** bitflags compiles
  (ADR 0184). **A library's trait's defaults, a number's `LowerHex` given a
  `Formatter`, and modules of one name in blocks:** a crate that uses its
  macro compiles, and runs as natively (ADR 0185).
- **A clone of std's iterator over an array,** chrono's `DelayedFormat`
  items (ADR 0181).
- **Generic code that writes to any `fmt::Write`,** chrono's, 7 refusals,
  and bitflags' `to_writer`, given a `Formatter` (ADR 0180).
- **A byte's ASCII tests and a slice's `split_first`,** chrono's (ADRs 0157,
  0153).
- **`Option::filter` and `map_or` of a value with a destructor,** chrono's
  (ADR 0179).
- **A value of an associated type where a type may have a destructor,**
  chrono's `Tz::Offset`, 55 refusals (ADR 0178).
- **A `NonZero` integer,** a pattern type `(u8) is 1..` in std, stopped
  chrono, at 333 places, and deranged (ADR 0177).
- **num-traits compiles,** after `Wrapping`, a float's methods, every
  integer's bits, defaults of `Self`, and a generic impl's constant of its
  parameters (ADRs 0156, 0175, 0176).
- **`{:p}` of a generic `T`:** thiserror compiles now (ADR 0174).
  **`f32::is_normal` and `classify`,** num-traits' (ADR 0122); **the bit
  methods of every integer and a float's bytes,** num-traits' `PrimInt` and
  `Float`, 189 refusals (ADR 0156). **`std::num::Wrapping`,** num-traits'
  (ADR 0175).
- **A path's `display()`,** thiserror's (ADR 0173); **`Option::map` of a
  value with a destructor and a generic `Option<T>`'s drop,** num-traits' and
  zerofrom's, which compiles now (ADR 0098); **`ParseIntError::kind()` and
  `{:?}` of `IntErrorKind`,** deranged's (ADR 0063).
- **Bytes to text,** `str::from_utf8`, `from_utf8_lossy` and the like:
  regex-syntax compiles now (ADR 0172). **`f32::to_int_unchecked`,**
  num-traits' (ADR 0171).
- **`u128` and `i128`** stopped num-traits, num-conv, zerofrom and
  serde_core; num-conv and time-core compile now (ADR 0171).
- **A `#![no_std]` crate's std items,** which rustc names `core::str::Chars`,
  weren't recognized: num-traits' `chars()`. **`size_hint()` of a generic
  iterator,** serde_core's, bounds that hold, a listed difference (ADR 0170).
- **`?` of a value with a destructor,** num-traits' `checked_pow` (ADR
  0098); **`char::from_u32_unchecked`,** utf8_iter's, which now compiles
  (ADR 0157); **`split_at_checked`,** writeable's (ADR 0153).
- **`size_hint()`** of an iterator of the crate's, utf8_iter's (ADR 0170);
  **a slice's `starts_with` and `eq_ignore_ascii_case` of bytes,**
  writeable's and uuid's (ADR 0153).
- **Two traits of one name for one type,** serde_core's `de::Error` and
  `ser::Error`, named alike (ADR 0133); **a user `AsMut` or `BorrowMut`,**
  smallvec's and arrayvec's (ADR 0169); **a slice's `split_at` and `get`
  of a range,** uuid's and writeable's (ADR 0153).
- **A user `Hash`** stopped semver, uuid, arrayvec and powerfmt (ADR 0168).
- **A user `Borrow`, and `Borrow::borrow` of a generic key,** stopped uuid,
  deranged and equivalent (ADR 0167).
- **A user `fmt::Write`** stopped anyhow, writeable and serde_core
  (ADR 0166).
- **A user `LowerHex`, `Pointer` and the like** stopped uuid and thiserror
  (ADR 0165).
- **A user `DoubleEndedIterator` or `ExactSizeIterator`** stopped 8
  (ADR 0164).
- **A generic trait method where a type may have a destructor** stopped
  9 (ADR 0163).
- **num-traits' supertrait dictionary names colliding** was an internal
  error (ADR 0106).
- **A `#![no_std]` crate's trait call crashed rust-js,** which looked up
  `alloc`'s `ToString` there: bitflags', num-traits'.
