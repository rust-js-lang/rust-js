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
| itertools 0.14.0 | compiles | measured 2026-11-11, after #1361 (std's ranges hashed as their derived `Hash` writes them, `Hashed::Range`): it compiles, 2 compiled, 0 refused; its probe is refused at "`the buffer std collects an iterator in place in` of a type parameter" (#1370); measured 2026-11-11, after #1346 (a map's class of a tuple of a type parameter composed of its parts', `$keyClassOf`): it compiles, 2 compiled, 0 refused; its probe is refused at `<Range<i32> as Hash>`'s evidence (#1361); measured 2026-11-11, after #1313 and #1348 (its `MapSpecialCaseFn` impls asked to carry a layout, ec99f10af's regression): under #1346 at `self.enumerate().minmax_by_key(..)`, a default calling the blanket impl's copy at `Enumerate<Self>`, whose callee asks the map class of a type built of it, which no finite set of facts can give; #1313 asks each impl that copies a default its calls' facts of `Self`, `multi_cartesian_product(self)`'s `IInPlace`, and its collects of `Self` take #1239's `IVecCollect`; measured 2026-11-08, after ADR 0914's amendment (#840): its closures compiled by a consumer, `Product`'s and `kmerge`'s, read from where they're made; its probe is refused at 9 type facts of type parameters (#934 and its kind); measured 2026-11-06, after ADR 0495's amendment (#828): what each default of its blanket impl makes is asked what it drops in the impl's terms, `sorted_unstable`'s `Vec::from_iter(self)` too; its probe is still refused first at "an iterator's own `last` given a dictionary of its own type parameter"; on 2026-11-05, after ADR 0145's amendment (#737): `multi_cartesian_product`'s collect of `MultiProductIter`s, a type built of its parameters, is given its layout by its caller, and `free::sorted` gives `Itertools` for `<I as IntoIterator>::IntoIter` what its impl asks; its probe is refused at "an iterator's own `last` given a dictionary of its own type parameter", `MultiProduct`'s, which asks that layout; on 2026-11-02, after ADRs 0202, 0687 and 0545 amended (#473), at a closure that holds part of a value with a destructor; on 2026-10-22, after ADR 0552's amendment, at `collect_vec()` of a `chunk_by` group's `copied()`, a `Group` whose `Drop` the library's `collect_vec` takes none of |
| indexmap 2.14.2 | runs | a `Map`, rustc builds it (ADR 0424) |
| bitflags 2.13.2 | runs | |
| smallvec 1.16.4 | runs | measured 2026-11-02, at 1.16.4, after ADRs 0145, 0099 and 0914 amended (#476): `size_of::<A::Item>()` of a library's dictionary, a `&mut usize` of a tuple given on, `.map(Self)` |
| arrayvec 0.7.8 | runs | measured 2026-11-02, after #468's fix |
| hashbrown 0.17.1 | runs | its maps std's, rustc builds it (ADR 0423) |
| thiserror 2.0.21 | runs | |
| anyhow 1.0.104 | refused | measured 2026-11-03, at its lock, after ADR 0454's amendment (#516): `{}` of a `dyn Error + Send + Sync` made where it's shown; its `Mut<'a, T>`, a `PhantomData<&'a mut T>` beside its pointer, compiles; its pointer casts (ADR 0562), `Backtrace` (ADR 0568) and `Display::fmt` on a held `Formatter` (ADR 0461) compile; `format_err`'s `Arguments::as_str` of its parameter is refused (ADR 0034, amended; #1091) |
| once_cell 1.21.4 | refused | `std::thread::current` called |
| semver 1.0.28 | compiles | measured 2026-10-21, after ADR 0562: its tagged pointers, inline identifiers and pointer constants compile; no probe runs it yet |
| uuid 1.29.0 (`std`, `v4`, `js`) | runs | measured 2026-11-10, its `js` through rust-js's own wasm-bindgen family (ADR 1112): 7 compiled, its probe as native; the Crater, 59 of its 60 tests as native, 1 refused (#1155) |
| getrandom 0.3.4 (`wasm_js`) | runs | measured 2026-11-10, its `wasm_js` backend through rust-js's own wasm-bindgen (ADR 1112), its `ptr_from_mut` polyfill given its place's pointer (ADR 1114): 8 compiled, its probe as native, `fill` and `fill_uninit`; the Crater, 6 of its 9 tests as native, 1 refused (`thread::spawn`), 2 refused at run time, `u32()` and `u64()`'s byte view of a `MaybeUninit<u32>` (#1191, which the Crater shows as refused at run time, #1192) |
| url 2.5.8 | blocked | measured 2026-11-03, at its lock: 14 of its 16 crates compile. yoke: raw pointers to erase lifetimes. utf_types 1.0.0, new to the graph through utf8_iter 1.1.0, at `core::ptr::from_ref` (refused the same way since before 3bbfc9f02: no regression). litemap compiles since 8afa0e975 (#514) |
| regex 1.13.1 | blocked | aho-corasick: memchr's `memmem::Finder`, which ADR 0529 doesn't ship; its unaligned reads compile (ADR 0562). regex-syntax 0.8.11, measured 2026-11-03: refused at a `&mut ast::parse::Parser` in a generic function's parameters. Until #511, rust-js crashed on it, a stack overflow (regex-syntax 0.8.8 at e18f3d9d1 too), that the script counted as neither compiled nor refused |
| rust_decimal 1.43.0 | runs | measured 2026-11-02, after ADR 0176's and 0914's amendments (#468): arrayvec's `ArrayVecImpl` impl it compiles has its `CAPACITY = CAP`, and its `a * b` is called through `decimalMul` |
| chrono 0.4.45 | runs | measured 2026-11-10 with its default features, `clock`, `std`, `oldtime` and `wasmbind`, its local time through rust-js's own js-sys (ADR 1112): 10 compiled, its probe as native; the Crater, 243 of its 272 tests as native, 9 refused (after #1158, #1159 and #1160), 20 of `cfg(unix)` not of wasm32; measured 2026-11-02, after ADR 0592's and 0051's amendments (#467): its `date` module's dictionaries, in a cycle with `time_delta`'s constants, and `format::parse`'s call of `parse_internal` |
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
- **A `#![no_std]` crate's generic `into()` or `collect()` crashed
  rust-js,** which looked up `alloc`'s `BTreeMap` there: powerfmt's,
  yoke's (#983).

## The top crates by downloads (#1174)

crates.io's most downloaded crates on 2026-11-10, of the first 300, each whose
library builds for `wasm32-unknown-unknown` with its default features and
whose package has its own `#[test]`s, no proc-macro crate and none of the
wasm-bindgen family rust-js serves itself (ADR 1112): with the rows above,
the first 100. Each at its newest release that day, its default features on,
for the Crater (ADR 1109) as much as for its graph.

A push's check, `bun scripts/crate-corpus.ts --check --sample`, asks every
row above and one in 8 of these, a hash of the commit and each crate's name
picking a new set each commit; the nightly asks them all. Each graph refused
somewhere is built again on every run: all 100 rows took 94 s with their
builds kept, 124 s without, where the rows above take 6 s and 30 s
(2026-11-10, on the Mac).

| Crate, one in 8 a push | Verdict | First refusals in its graph |
|---|---|---|
| syn 3.0.10 | blocked | measured 2026-11-10: proc-macro2@1.0.107: rust-js does not support statics of type `std::sync::Once` yet |
| rand_core 0.10.1 | refused | measured 2026-11-10: rand_core@0.10.1: rust-js does not support `size_of` of a type parameter yet |
| libc 0.2.191 | compiles | measured 2026-11-10: compiles, no probe yet |
| quote 1.0.47 | blocked | measured 2026-11-10: proc-macro2@1.0.107: rust-js does not support statics of type `std::sync::Once` yet |
| proc-macro2 1.0.107 | refused | measured 2026-11-10: proc-macro2@1.0.107: rust-js does not support statics of type `std::sync::Once` yet |
| cfg-if 1.0.5 | compiles | measured 2026-11-10: compiles, no probe yet |
| memchr 2.8.3 | compiles | measured 2026-11-10: compiles, no probe yet |
| regex-syntax 0.8.11 | refused | measured 2026-11-10: regex-syntax@0.8.11: rust-js does not support a `&mut ast::parse::Parser` inside a generic function's parameters or result yet |
| serde 1.0.229 | compiles | measured 2026-11-10: compiles, no probe yet |
| unicode-ident 1.0.27 | compiles | measured 2026-11-10: compiles, no probe yet |
| rand_chacha 0.10.0 | blocked | measured 2026-11-10: rand_core@0.10.1: rust-js does not support `size_of` of a type parameter yet; zerocopy@0.8.63: rust-js: generated trait implementation name `_core__marker__PhantomData_Src___core__marker__PhantomData_Dst__TransmuteRefStaticAssert` collides; put the implementations in separate modules |
| itoa 1.0.19 | refused | measured 2026-11-10: itoa@1.0.19: rust-js does not support calling `core::slice::<impl [T]>::get_unchecked` yet |
| serde_json 1.0.154 | blocked | measured 2026-11-10: itoa@1.0.19: rust-js does not support calling `core::slice::<impl [T]>::get_unchecked` yet; zmij@1.0.23: rust-js does not support addressable memory yet: raw addresses of statics |
| autocfg 1.5.1 | refused | measured 2026-11-10: autocfg@1.5.1: rust-js does not support calling `std::ffi::OsString::new` yet |
| log 0.4.35 | refused | measured 2026-11-10: log@0.4.35: rust-js does not support addressable memory yet: a `transmute` of `usize` as `LevelFilter`, which aren't numbers alone |
| cc 1.11.1 | blocked | measured 2026-11-10: find-msvc-tools@0.1.15: rust-js does not support calling `std::env::split_paths` yet |
| regex-automata 0.4.18 | blocked | measured 2026-11-10: aho-corasick@1.1.5: rust-js does not support calling `memchr::memmem::Finder::<'n>::new` yet; regex-syntax@0.8.11: rust-js does not support a `&mut ast::parse::Parser` inside a generic function's parameters or result yet |
| aho-corasick 1.1.5 | refused | measured 2026-11-10: aho-corasick@1.1.5: rust-js does not support calling `memchr::memmem::Finder::<'n>::new` yet |
| digest 0.11.3 | blocked | measured 2026-11-10: hybrid-array@0.4.15: rust-js does not support generic associated types with bounds yet |
| block-buffer 0.12.1 | blocked | measured 2026-11-10: hybrid-array@0.4.15: rust-js does not support generic associated types with bounds yet |
| ryu 1.0.23 | compiles | measured 2026-11-10: compiles, no probe yet |
| parking_lot_core 0.10.0 | refused | measured 2026-11-10: parking_lot_core@0.10.0: rust-js does not support calling `std::thread::LocalKey::<T>::try_with` yet |
| parking_lot 0.13.0 | blocked | measured 2026-11-10: lock_api@0.5.0: rust-js does not support addressable memory yet: a raw pointer to what a `&mut` refers to, which JS holds in a box of a copy: a function whose signature has a raw pointer in its result or another parameter is given its place's pointer (ADR 1114); parking_lot_core@0.10.0: rust-js does not support calling `std::thread::LocalKey::<T>::try_with` yet |
| strsim 0.11.1 | compiles | measured 2026-11-10: compiles, no probe yet |
| bytes 1.12.1 | refused | measured 2026-11-10: bytes@1.12.1: rust-js does not support a borrow of where an object is stored, in generic code yet: `<*mut () as core::sync::atomic::AtomicPrimitive>::Storage`'s variants hold their values at different offsets in it, which its JS, the value itself, doesn't say (#1114) |
| pin-project-lite 0.2.17 | compiles | measured 2026-11-10: compiles, no probe yet |
| http 1.5.0 | blocked | measured 2026-11-10: itoa@1.0.19: rust-js does not support calling `core::slice::<impl [T]>::get_unchecked` yet; bytes@1.12.1: rust-js does not support a borrow of where an object is stored, in generic code yet: `<*mut () as core::sync::atomic::AtomicPrimitive>::Storage`'s variants hold their values at different offsets in it, which its JS, the value itself, doesn't say (#1114) |
| version_check 0.9.5 | refused | measured 2026-11-10: version_check@0.9.5: rust-js does not support calling `std::process::Command::new` yet |
| sha2 0.11.1 | blocked | measured 2026-11-10: const-oid@0.10.2: rust-js does not support addressable memory yet: values of type `*const ObjectIdentifierRef`; hybrid-array@0.4.15: rust-js does not support generic associated types with bounds yet |
| tokio 1.54.1 | refused | measured 2026-11-10: tokio@1.54.1: rust-js does not support calling `std::hash::RandomState::new` yet |
| lock_api 0.5.0 | refused | measured 2026-11-10: lock_api@0.5.0: rust-js does not support addressable memory yet: a raw pointer to what a `&mut` refers to, which JS holds in a box of a copy: a function whose signature has a raw pointer in its result or another parameter is given its place's pointer (ADR 1114) |
| lazy_static 1.5.1 | refused | measured 2026-11-10: lazy_static@1.5.1: rust-js does not support a `pub use` of `lazy::std::sync::ONCE_INIT` yet: it's in no JS module of rust-js's to be re-exported from |
| winnow 1.0.5 | refused | measured 2026-11-10: winnow@1.0.5: rust-js does not support user implementations of `std::borrow::ToOwned` yet |
| idna 1.1.0 | blocked | measured 2026-11-10: litemap@0.8.3: rust-js does not support the buffer std collects an iterator in place in of `core::iter::FilterMap<<S as store::StoreIntoIterator<K, V>>::KeyValueIntoIter, {closure@/Users/tunguyen/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/litemap-0.8.3/src/map.rs:833:29: 833:37}>`, which no caller gives yet; utf_types@1.0.0: rust-js does not support addressable memory yet: a pointer read as another type's |
| zerocopy 0.8.63 | refused | measured 2026-11-10: zerocopy@0.8.63: rust-js: generated trait implementation name `_core__marker__PhantomData_Src___core__marker__PhantomData_Dst__TransmuteRefStaticAssert` collides; put the implementations in separate modules |
| scopeguard 1.2.0 | refused | measured 2026-11-10: scopeguard@1.2.0: rust-js does not support calling `std::thread::panicking` yet |
| toml 1.2.1+spec-1.1.0 | blocked | measured 2026-11-10: toml_writer@1.2.0+spec-1.1.0: rust-js does not support implementation evidence for `<std::borrow::Cow<'_, str> as std::convert::AsRef<str>>` yet; winnow@1.0.5: rust-js does not support calling `core::slice::<impl [T]>::get_unchecked` yet |
| slab 0.4.12 | refused | measured 2026-11-10: slab@0.4.12: rust-js does not support `clone_from` of items whose `clone_from` may be the crate's, inside them or generic yet |
| futures-util 0.3.34 | blocked | measured 2026-11-10: futures-core@0.3.34: rust-js does not support a `Pin` of this pointer yet; futures-task@0.3.34: rust-js does not support statics of type `noop_waker::noop_waker_ref::SyncRawWaker` yet |
| futures-task 0.3.34 | refused | measured 2026-11-10: futures-task@0.3.34: rust-js does not support statics of type `noop_waker::noop_waker_ref::SyncRawWaker` yet |
| hyper 1.12.0 | blocked | measured 2026-11-10: itoa@1.0.19: rust-js does not support calling `core::slice::<impl [T]>::get_unchecked` yet; bytes@1.12.1: rust-js does not support a borrow of where an object is stored, in generic code yet: `<*mut () as core::sync::atomic::AtomicPrimitive>::Storage`'s variants hold their values at different offsets in it, which its JS, the value itself, doesn't say (#1114) |
| fastrand 2.5.0 | refused | measured 2026-11-10: fastrand@2.5.0: rust-js does not support calling `std::thread::LocalKey::<T>::try_with` yet |
| typenum 1.20.1 | compiles | measured 2026-11-10: compiles, no probe yet |
| miniz_oxide 0.9.1 | refused | measured 2026-11-10: miniz_oxide@0.9.1: rust-js does not support values of type `&mut &mut [u8]` yet |
| crossbeam-utils 0.8.23 | refused | measured 2026-11-10: crossbeam-utils@0.8.23: rust-js does not support statics of type `[cache_padded::CachePadded<atomic::seq_lock::SeqLock>; LEN]` yet |
| percent-encoding 2.3.2 | refused | measured 2026-11-10: percent-encoding@2.3.2: rust-js does not support addressable memory yet: a pointer into what JS may hold a copy of |
| generic-array 1.4.5 | refused | measured 2026-11-10: generic-array@1.4.5: rust-js does not support generic associated types with bounds yet |
| shlex 2.0.1 | compiles | measured 2026-11-10: compiles, no probe yet |
| http-body 1.1.0 | blocked | measured 2026-11-10: itoa@1.0.19: rust-js does not support calling `core::slice::<impl [T]>::get_unchecked` yet; bytes@1.12.1: rust-js does not support a borrow of where an object is stored, in generic code yet: `<*mut () as core::sync::atomic::AtomicPrimitive>::Storage`'s variants hold their values at different offsets in it, which its JS, the value itself, doesn't say (#1114) |
| futures-channel 0.3.34 | blocked | measured 2026-11-10: futures-core@0.3.34: rust-js does not support a `Pin` of this pointer yet |
| ppv-lite86 0.2.21 | blocked | measured 2026-11-10: zerocopy@0.8.63: rust-js: generated trait implementation name `_core__marker__PhantomData_Src___core__marker__PhantomData_Dst__TransmuteRefStaticAssert` collides; put the implementations in separate modules |
| tracing-core 0.1.36 | blocked | measured 2026-11-10: once_cell@1.21.4: rust-js does not support a borrow of where an object is stored, in generic code yet: `<*mut imp::Waiter as std::sync::atomic::AtomicPrimitive>::Storage`'s variants hold their values at different offsets in it, which its JS, the value itself, doesn't say (#1114) |
| tracing 0.1.44 | blocked | measured 2026-11-10: once_cell@1.21.4: rust-js does not support a borrow of where an object is stored, in generic code yet: `<*mut imp::Waiter as std::sync::atomic::AtomicPrimitive>::Storage`'s variants hold their values at different offsets in it, which its JS, the value itself, doesn't say (#1114) |
| toml_edit 0.25.19+spec-1.1.0 | blocked | measured 2026-11-10: toml_writer@1.2.0+spec-1.1.0: rust-js does not support implementation evidence for `<std::borrow::Cow<'_, str> as std::convert::AsRef<str>>` yet; winnow@1.0.5: rust-js does not support user implementations of `std::borrow::ToOwned` yet |
| rustc-hash 2.1.3 | refused | measured 2026-11-10: rustc-hash@2.1.3: rust-js does not support user implementations of `core::hash::BuildHasher` yet |
| tempfile 3.27.0 | blocked | measured 2026-11-10: fastrand@2.5.0: rust-js does not support calling `std::thread::LocalKey::<T>::try_with` yet; once_cell@1.21.4: rust-js does not support a borrow of where an object is stored, in generic code yet: `<*mut imp::Waiter as std::sync::atomic::AtomicPrimitive>::Storage`'s variants hold their values at different offsets in it, which its JS, the value itself, doesn't say (#1114) |
| nix 0.31.3 | refused | measured 2026-11-10: nix@0.31.3: rust-js: the metadata rustc loaded for `cfg_if` isn't of the build its JS was made from: rebuild `cfg_if`, and use the metadata rust-js writes beside its JS |
| h2 0.4.20 | blocked | measured 2026-11-10: futures-core@0.3.34: rust-js does not support a `Pin` of this pointer yet; futures-sink@0.3.34: rust-js does not support values of type `&mut core::pin::Pin<P>` yet |
| rustls-webpki 0.103.16 | blocked | measured 2026-11-10: zeroize@1.9.1: rust-js does not support `size_of` of a type parameter yet |
| unicode-width 0.2.3 | compiles | measured 2026-11-10: compiles, no probe yet |
| untrusted 0.9.0 | compiles | measured 2026-11-10: compiles, no probe yet |
| byteorder 1.5.0 | refused | measured 2026-11-10: byteorder@1.5.0: rust-js does not support addressable memory yet: a raw pointer to what a `&mut` refers to, which JS holds in a box of a copy: a function whose signature has a raw pointer in its result or another parameter is given its place's pointer (ADR 1114) |
| tokio-util 0.7.20 | blocked | measured 2026-11-10: futures-sink@0.3.34: rust-js does not support values of type `&mut core::pin::Pin<P>` yet; futures-core@0.3.34: rust-js does not support a `Pin` of this pointer yet |
| futures 0.3.34 | blocked | measured 2026-11-10: futures-core@0.3.34: rust-js does not support a `Pin` of this pointer yet; futures-io@0.3.34: rust-js does not support values of type `&mut core::pin::Pin<P>` yet |
| pkg-config 0.3.34 | refused | measured 2026-11-10: pkg-config@0.3.34: rust-js does not support calling `std::path::Path::exists` yet |
| clap_builder 4.6.7 | blocked | measured 2026-11-10: is_terminal_polyfill@1.70.2: rust-js does not support calling `std::io::IsTerminal::is_terminal` yet; anstyle-query@1.1.5: rust-js does not support `as_deref` of a `std::ffi::OsString` yet |
| webpki-roots 1.0.11 | compiles | measured 2026-11-10: compiles, no probe yet |
| darling_core 0.24.1 | blocked | measured 2026-11-10: proc-macro2@1.0.107: rust-js does not support statics of type `std::sync::Once` yet |
| darling 0.24.1 | blocked | measured 2026-11-10: proc-macro2@1.0.107: rust-js does not support statics of type `std::sync::Once` yet |
| anstyle 1.0.14 | compiles | measured 2026-11-10: compiles, no probe yet |
| reqwest 0.13.5 | blocked | measured 2026-11-10: futures-core@0.3.34: rust-js does not support a `Pin` of this pointer yet; futures-task@0.3.34: rust-js does not support statics of type `noop_waker::noop_waker_ref::SyncRawWaker` yet |
| futures-executor 0.3.34 | blocked | measured 2026-11-10: futures-core@0.3.34: rust-js does not support a `Pin` of this pointer yet; futures-task@0.3.34: rust-js does not support statics of type `noop_waker::noop_waker_ref::SyncRawWaker` yet |
| rustc_version 0.4.1 | refused | measured 2026-11-10: rustc_version@0.4.1: rust-js does not support calling `std::ffi::OsStr::is_empty` yet |
| synstructure 0.14.0 | blocked | measured 2026-11-10: proc-macro2@1.0.107: rust-js does not support statics of type `std::sync::Once` yet |
| tinyvec 1.13.3 | refused | measured 2026-11-10: tinyvec@1.13.3: rust-js does not support `size_hint()` of a `core::slice::IterMut<'_, T>`, from its MIR yet |
| nom 8.0.0 | refused | measured 2026-11-10: nom@8.0.0: rust-js does not support `size_of` of a type parameter yet |
| httparse 1.10.1 | refused | measured 2026-11-10: httparse@1.10.1: rust-js does not support values of type `&mut &mut [Header<'_>]` yet |
