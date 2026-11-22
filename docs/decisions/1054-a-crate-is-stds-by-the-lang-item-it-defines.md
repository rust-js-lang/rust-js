# 1054. A crate is std's by the lang item it defines, not by its name

Status: Accepted (#455, P2, 2026-11-02). Amends
[0331](0331-dyn-any.md): a `TypeId` of a crate named as one of std's is,
the crate compiled too, has its hash beside its name, and std's never;
and [0561](0561-std-bodies.md): `@rust-js/std` is the library of std's
crates by its package's name, not theirs.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior is kept. A program's crate named `core` is that crate in
Rust, its items its own.

## Context

A workspace's path package named `core`, with `pub fn double(x: i32) ->
i32`, called from another crate as `core::double(21)`, was refused, "rust-js
does not support calling `core::double` yet"; renamed `leaf_core`, it
worked. Natively it builds: `--extern core=..` puts the program's crate in
the extern prelude, and std's `core` stays std's, which std's macros and
`$crate` paths reach.

rust-js decided what's std's by a crate's name, `tcx.crate_name(krate) ==
sym::core`, in fifty places, and by a path, `def_path_str`'s
`core::mem::take` made `std::mem::take`. So the program's `core` was
std's: a call of it was a std function rust-js has no lowering of,
refused, and worse, one named as a std function is, `core::hint::black_box`
or `core::mem::take`, was taken for it, a different answer. The library
map knew `@rust-js/std` by the names `core`, `alloc` and `std` too, which
the program's library of that name replaced. A crate named `test` was
libtest's `test::black_box`, by name, the same.

## Decision

**std's crates are known by the lang item only each defines, not by
name.** `recognition::std_crate(tcx, krate)` is the one answer:
`core` is the crate that defines `Sized`, `alloc` the one that defines
`Box` (`owned_box`), `std` the one that defines `start`, which runs a
program's `main`. A lang item is defined only under
`#![feature(lang_items)]`, which stable Rust refuses, so no program's
crate defines one, whatever its name. `std_crate_num(tcx, StdCrate::Core)`
is the crate, found the same way.

Each question of std's crates asks it:

- `is_std_crate(tcx, id)` and `of_std_crate(tcx, id, StdCrate::Core)`
  replace every `crate_name(..) == sym::core`, and `is_std_item`, the same
  question asked twice, is gone.
- `std_path(tcx, id)` is std's path of a std item and empty of any other,
  which no std path is: a program's `std::process::exit` is never std's.
- A lookup by path in std's crate, `registry::by_path`, takes a
  `StdCrate`.
- libtest is the `test` crate rustc's sysroot holds, not any crate of
  that name.
- `Dependencies` keeps `@rust-js/std` by `STD_PACKAGE`, which no crate is
  named, and the front end finds a crate's library by
  `library::library_name`, std's crates' that, any other's its name.
- A type of a crate named as another crate the session sees is, the
  crate compiled too, has its `TypeId`'s hash beside its name (ADR 0331),
  so a `core::option::Option` of the program's is never std's `Option` to
  a downcast; std's own types never do, their names the same in every
  crate, `alloc::string::String`, as `@rust-js/runtime` reads them.

So a program's crate named `core`, `alloc`, `std` or `test` is an
ordinary crate, compiled and called as any other, and std's crates are
std's beside it.

## Why

- **One owner**: whether a crate is std's is `std_crate`'s answer; no
  module compares a crate's name with std's, and no path is std's but
  through `std_path`, which asks it.
- **rustc's own identity**: rustc finds `Sized`, `Box` and `start` by
  their lang items, the crate that defines each. A name is what a
  program chooses; a crate's `extern crate core as other` keeps it
  std's.
- **Alternatives**: the crate a sysroot holds (`in_sysroot`) is a file's
  path, and rust-js's own sysroot of a `core` built to unwind (#424) is
  another. A diagnostic item would serve as well, but a lang item is what
  the compiler itself can't do without: `Sized` and `Box` are where they
  are in every release. A refusal of a crate named `core` would refuse a
  valid program.

## Tested

- `test/crates/pairs/crate_named_core`: a crate named `core` with
  `double`, a struct's `Display`, and `mem::take`, `hint::black_box`,
  `iter::once`, `any::type_name`, `char::from_digit` of its own and a
  `option::Option` downcast, beside std's of each, against native Rust.
- `test/crates/triples/crates_named_alloc_and_test`: crates named `alloc`,
  with `fmt::format` and `string::String` of its own, and `test`, with
  `black_box`; `test/crates/triples/crate_named_std`: a crate named `std`,
  given to a `#![no_std]` crate, with `process::exit`, `mem::take` and
  `hint::black_box` of its own, used by a crate whose `std` is std's.
  A crate named `std` is given only to a `#![no_std]` crate, natively too:
  another's `std` is the one its prelude reads.
- A crate rust-js didn't compile, its metadata alone, named `test` or
  `std`: a call of its `black_box` or its `process::exit` is refused, as
  any such crate's is. A library's call is its library's before std's are
  asked, so this is where `std_path` and libtest's identity are seen.
- `test/architecture.test.ts` refuses `sym::core` and `"core" |` of a
  crate's name in the front end and the library contract, but in
  `std_path`.
- Mutations: std's crates by name again, `std_path` of any crate's item,
  `@rust-js/std` by std's crates' names, both where it's found and where
  it's kept, libtest by name, std's `TypeId`s hashed beside a crate of
  their name, and the crate compiled left out of who shares its name.
  `registry::by_path` by the first crate of std's name has none: rustc
  loads std's crates, injected, before any crate a program names, so
  the first `core` is std's.

## Amended: std's path of an item is std's own visible parent (#2314, 2026-11-22)

`std_path_parent`, the module std's path writes an item in, asked rustc's
`visible_parent_map`. That query does a breadth-first walk of every
crate's public modules. It cost 37 ms in each of the React port's
compiles, and its answer for a std item depended on the program's crate
graph: a dependency that re-exported a std item nearer its root than std
does became that item's parent. `std_visible_parents` now does the same
walk, with the same rules (public children only; a child re-exported as
`_`, by a `#[doc(hidden)]` module or by a `#[doc(hidden)]` re-export
ranked after any other parent), but starts only from std's crates (this
ADR's `std_crate`). That ranking stays even though no output yet shows it.
Without it, in a crate that names `extern crate core`,
`core::panicking::panic_fmt` and `panic_display` get other parents than
rustc gives them, and other answers to what's under std's modules (453 of
the crate corpus's shadowed calls). The parent std writes an item in,
rather than the module it's defined in, does show in output:
`core::iter::Take`, defined in `core::iter::adapters::take`, is std's
`iter::Take` (mutation `std-path-parent-defined-in`, caught by
drop_order_through_take_adapters' snapshot). An item that isn't std's, and a std crate's root, have
no std parent. A temporary shadow check compared both answers and found no
difference, in recognitions and in std items' parents: 11.68M calls in the
corpus, 5.58M in the React port and 7.38M in the crate corpus.

The walk is still forced wherever a type or a path is printed, so a
lowering prints only where it says something. The session's forgotten
types (#1665) are printed only when some type is forgotten, and a drop
compared with rustc's elaboration prints its function's path only in a
verdict it states. A timed compile (`RUST_JS_PHASE_TIMES`) lists each such
walk its lowering forced, and test/phase-times.test.ts holds a lowering of
std's iterators, adapters and strings to none. The React port's compiles
force none, 30.8 CPU s before and 30.0 after. 78 corpus cases still
print in lowering, through other callers (#2321).
