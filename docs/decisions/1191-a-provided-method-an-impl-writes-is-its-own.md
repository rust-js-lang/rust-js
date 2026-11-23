# 1191. A provided method an impl writes is its own, through generic code too

Status: Accepted (2026-11-24, the lead's decision for #2414, step 1, decided
for the user overnight). Amends [0185](0185-library-trait-defaults.md)
and [0561](0561-std-bodies.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): R. Which impl's method a
call runs is observable, so the call runs the one Rust's dispatch picks.

## Context

A std trait's provided method compiled into `@rust-js/std` (ADR 0561),
called on a type parameter or a `dyn`, was that default function over
`Self`, `bufRead_readLine(r, buf, RBufRead, ..)`, whatever impl the value's
type had. That holds only where no impl writes its own. Rust's dispatch
picks the impl's own where it writes one, and many do:

- a crate may write any stable provided method: a reader's `read_line`, a
  `Seek`'s `rewind`, an iterator's `partition`. Generic code ran std's
  default: `read_line` read the buffer where the override ran, `rewind` left
  the position, `partition` gave std's sides (#2414);
- std's own impls write `Extend`'s unstable `extend_one`, `extend_reserve`
  and `extend_one_unchecked`: a pair's, which splits each item between its
  sides, a `Vec`'s `reserve`. std's compiled `unzip()` (#2395) reached them
  through `default_extend` and recursed, as a pair's default `extend_one`
  is its `extend`.

## Decision

**One owner says whether an impl in reach writes its own of a provided
method; where one does, a call on a type parameter or a `dyn` is Rust's
dispatch.**

- `recognition/overrides.rs` `overridden_in_reach(tcx, provided)`: whether
  any impl rustc loads, std's, the crate's or a library's, writes it,
  `impl_item_implementor_ids` of each of its trait's impls. A crate built on
  this one, which may write one too, is #2414's second step (its manifest).
- `in_std_dictionary` asks it, of the methods compiled into `@rust-js/std`,
  so each dictionary of the trait carries the method its impl writes: the
  crate's own (impls.rs's library-default path) and std's compiled impls'
  (`tuple2Extend`'s). A hand-made dictionary of a std collection carries
  what its std impl writes observably: `extendReserve`, its `reserve`
  (`traits/dictionaries/extend_entries.rs`), only where rustc says the
  impl writes `extend_reserve`, refused for a type with no form of it.
  std's `extend_one`, a collection's `push` or `insert`, is what the
  default, `extend(Some(item))`, does to it, so it carries none.
- traits/calls.rs asks it too. Where it says no, the call is the default
  over `Self` as before. Where it says yes, `$provided(dictionary, key,
  default, entry, byDefault)`: the dictionary's own where its impl writes
  one, given `entry`, what that entry takes (`through_dictionary`'s, or a
  `dyn`'s pair and its rest, `dyn_entry`'s); else the default, given
  `byDefault`, what it takes (`compiled_default_args`, the one list of the
  default's call). `key` is `bindings::fn_name`, the naming owner's name of
  the entry. Each list is names, dictionaries and drops, built once, so
  building both runs nothing.
- `DICTIONARY_METHODS` stays the one list of what a byte source writes;
  `$readerDefaults` and `$seekDefaults` are untouched.

**What isn't here.** An `Iterator`'s provided method: generic code holds an
iterator as a JS iterator (ADR 0061), with no `Iterator` dictionary, and
in-place consumers, `$partition`, never reach traits/calls.rs. A crate
iterator's own steps ride on its value (`OwnMethods`), so #2414's
`partition`/`peekable` case is #2112 Change A step 3 phase 3's: `OwnMethods`
carries each provided method `overridden_in_reach` names.

## Why

- **Rust's rule, one owner.** Which impl runs is what rustc says of the
  impls it loads; nothing restates it per trait.
- **Output moves only where something writes its own.** Where nothing does,
  the call is the default over `Self`, as it was.
- **One helper of the dispatch alone.** `$provided` encodes "the impl's own,
  else the default", no std body: std's defaults stay compiled from std.

## Alternatives

- **Every dictionary of the trait completed with std's default.** The
  runtime's hand-made dictionaries can't import `@rust-js/std`'s compiled
  defaults, so it meant runtime copies of std's bodies or a third list.
- **Every stable provided method through the dictionary.** Sound, but every
  generic `it.peekable()` and `r.read_line(..)` would move in every program.

## Consequences

- std's forwarders write `read_line`, `read_until`, `skip_until` and
  `rewind` (`&mut B`, `Box`, `Empty`, `Chain`, `StdinLock`, `Arc`), so
  generic code's calls of them are `$provided` in every program.
- std's compiled `SpecExtend::extend` calls `$provided` of its collection's
  `extendReserve` and `extendOne`.
- Corpus, against native: `provided_methods_a_type_writes_are_its_own` (a
  crate's `read_line`, through a type parameter and a `dyn`),
  `rewind_a_type_writes_is_its_own`,
  `provided_io_methods_of_std_readers_by_generic_code`,
  `extend_reserve_of_sides_generic_code_fills` (capacities of a `Vec` and a
  `VecDeque` side, of a `TrustedLen` and a `filter`),
  `extend_reserve_of_an_endless_source_panics_in_order` (hashbrown's room
  check before a `Vec`'s, and after), and
  `hash_set_extend_reserves_half_once_it_holds_items` (wasm32: hashbrown's
  `div_ceil(2)` of a set that holds items). Its empty-set branch, the whole
  lower bound, can't be told from half against native, as wasm32 aborts on
  the panic and a 64-bit native has room for either: so the rule is pinned
  to hashbrown's source where it's mirrored, a `std-source-pin` line in
  `extend_entries.rs`, which test/std-specializations.test.ts checks as it
  checks `OBSERVABLY_DEFAULT`'s rows.
- Mutations: `provided-overridden-nowhere`, `overridden-provided-not-in-dictionary`,
  `provided-overridden-shortcut`, `provided-key-not-its-entry-name`,
  `provided-dyn-receiver-its-box`, `provided-always-the-default`,
  `extend-reserve-entry-never`, `hash-extend-reserve-whole`,
  `hash-extend-reserve-unchecked`.
