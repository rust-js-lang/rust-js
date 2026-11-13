# 0109. Pin a stable release: rust-js checks code as the Rust most people run does

Status: Accepted. Amends [0003](0003-pin-nightly-toolchain.md): the pin is a
stable release, `1.100.0` (amended below, from `1.99.0`), where it was
`nightly-2026-03-25`. Amended by [0110](0110-stable-syntax.md): rust-js's syntax is stable Rust's, so its
session allows no unstable feature, and rustc refuses a crate's own itself.

Case: N, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

rust-js checks a program with the rustc it links, so its pin is the Rust a
user's code can be. Rust is backward compatible, so a newer rustc takes all
code an older one did: what matters is not being older than the Rust users
run. Most of them run the latest stable, which rustup keeps them on, and
crates declare a minimum Rust version at or below it. The pin was a nightly
of 1.96 when stable was 1.98: code that used what 1.97 and 1.98 stabilized
was an error in rust-js.

A nightly was pinned because rustc's internal crates, `rustc_private`, are a
nightly's. A stable release ships them too, in its `rustc-dev` component, and
`RUSTC_BOOTSTRAP` lets a crate use them.

## Decision

**The pin is a stable release, `1.100.0`, and moves with each one.**

- **rust-js's own crate uses rustc's internals**, by `RUSTC_BOOTSTRAP=rust_js`
  in `.cargo/config.toml`: that crate's name alone, so no other crate is
  built with unstable features.
- **Its syntax is rustc's unstable features** (ADR 0039): `register_tool` and
  the rest, which a stable release allows no crate. rust-js's session allows
  them, and refuses a crate's own `#![feature]`, as that release does, with
  its error, E0554: a program is stable 1.100.0's Rust. `RUSTC_BOOTSTRAP=1`
  allows it, as for rustc. A crate may name rust-js's own features, which
  it turns on anyway.
- **The binding crates are built so too**, each by its build script:
  `RUSTC_BOOTSTRAP=js`, `webapi`, `react`. Since ADR 0112, rust-js compiles
  them, and they need none.
- **The tests' programs, and rustc's**, which use rustc's features natively
  too, are built with `RUSTC_BOOTSTRAP=1`: `.env.test`, which `bun test`
  loads for every process it starts, and the rustc suite, as rustc's own CI
  runs its tests.
- **The playground's rustc is the release's source**, its version stamps the
  release's, `1.100.0 (0ef1bc386 2026-11-09)` on the `stable` channel, or it
  refuses the shipped `core`'s metadata. Its lockfile is rustc's own, with
  rust-js's crates' where they need a newer version.

## Why

- **The release is the Rust users run**, so rust-js takes what their
  `cargo build` takes, and refuses what it refuses, with its words.
- **A release is tested**, where a nightly is a day's snapshot.
- **The escape hatch is narrow**: the one crate, rust-js's, uses internals,
  and a user's code gets no feature it didn't have on stable.

## Consequences

- **The upgrade from 1.96**: instantiating a generic value now returns it
  marked as maybe needing normalization, `Unnormalized`, which rust-js takes
  as it was, `.skip_normalization()`; an alias type's kind carries its
  `DefId`; `EarlyBinder::bind` and `Ty::new_projection` take more.
- **A `TryFromIntError` is its message**, as each parse error is (ADR 0063):
  1.98's shows its kind, `TryFromIntError(PosOverflow)` where 1.96's was
  `TryFromIntError(())`, and since 1.99 its message tells the kind too,
  "number too large to fit in target type" or "too small", where it was one
  sentence for both. `{:?}` takes the kind from the message, as a
  `ParseIntError`'s does. `==` of a parse error, which its derived
  `PartialEq` compares by kind, is in.
- **The playground's patch for dynamic libraries** moved with rustc's code,
  to `rustc_metadata/src/host_dylib.rs`.
- **Found by rustc's tests at 1.98.1**, as the upgrade's own:
  - `size_of_val` of a generic `async fn`'s future is laid out as codegen
    lays it out, in a fully monomorphic environment: in the function's,
    1.98 finds it too generic.
  - An externally implementable item, 1.98's `#[eii] static HELLO: u64;`,
    which the linker makes another item, is refused: its JS read a name
    nothing defines.
  - 1.98's `assert_eq!` has a block of its own, so a temporary of its
    operands ends with that block, not the statement: one with a destructor,
    `assert_eq!(BAR.0, 456)` of a `const BAR: A` with a `Drop`, is refused,
    where rust-js drops only at a statement's end or a `let`'s.
- **rustc's tests are the release's**: rustc renamed and took out some,
  so a bless of another rustc than the inventory's takes the new ones, and
  a known failure no longer a test goes with its diff.
- **The playground's rustc and oxc would share a `hashbrown`** now, 0.17:
  rustc's `nightly` feature, for `may_dangle` in drop checks, which rustc's
  arenas need, would make oxc's std's unstable `Allocator`, which oxc's arena
  isn't. So the playground's rustc takes the same release from hashbrown's
  repository, `v0.17.0`, by a patch: to cargo, a package of its own.
- **The upgrade to 1.99**: rustc's lang items moved to `rustc_attr_ir`,
  which `rustc_hir::attrs` re-exports; `LocalModDefId` is `LocalModId`;
  `predicates_of` is `clauses_of`, its list `clauses`. The tools rustc
  knows are two lists now, an attribute's and a lint's: `rust_js` is an
  attribute's. A function item's generic args are bound now: a body's
  never have bound vars, so rust-js reads them unbound, `fn_def`, as
  rustc's own MIR building does.
- **1.99's messages**: a radix past 36 panics naming
  `from_ascii_bytes_radix`, where it was `from_ascii_radix`; `{:?}` shows
  halfwidth katakana's voiced marks, U+FF9E and U+FF9F, as they are,
  where it escaped them as the combining marks Unicode makes them. A
  character Unicode says to ignore, U+FFA0 say, a letter, it escapes, as
  rust-js now does too, which a test of these found.
- **A JS method named as libc's functions are**, `open`, `close`,
  `write`, is one in native code of the runtime's symbols, which 1.99's
  `invalid_runtime_symbol_definitions` checks, an error: the webapi crate
  allows it, as it allows `clashing_extern_declarations`, for a native
  `cargo check` of an app.
- **The playground needs one patch fewer**: 1.99's rustc grows no stack,
  `ensure_sufficient_stack` is gone, so `stacker`, and `psm` with its
  precompiled object, which Apple's `ar` couldn't archive, are too.
- **Found by rustc's tests at 1.99.0**: `IntoIterator` of a
  `Box<[T; N]>`, 1.99's, which `Box::new([1, 2]).into_iter()` now calls
  where it took a `&` of the array. A box is its array, so by value it is
  the array's; through a `&mut`, a handle on the box (ADR 0099), the
  handle's place, and `for n in &mut b` of numbers an index loop, as of an
  array.
- **Still unstable**: rust-js's syntax, which a plain rustc refuses without
  `RUSTC_BOOTSTRAP`. Moving it to stable Rust is to come.

## Amendment: the upgrade to 1.100.0 (2026-11-13)

The pin moved from 1.99.0 to 1.100.0, released 2026-11-12. Clippy's commit
moved with it to rust-clippy `e80eea1b01`, whose tree is 1.100's
`src/tools/clippy` where it branched, rust-lang/rust@fd986958, byte for
byte (ADR 0888). The playground's stamps are 1.100's, and it builds
`rustc_structures`, a crate 1.100 split out of `rustc_session`. Its patch
for dynamic libraries is re-cut for 1.100's `rustc_metadata`, the same
change, and its lockfile starts from 1.100's rustc's. `rustc_attr_ir`
includes the attribute docs' examples, `tests/ui/attributes/doc_examples`,
so the sparse checkout reads them too: its paths are one list,
`scripts/toolchain.ts sources`, which `wasm/build.sh` (each run, a
checkout made for 1.99 too), CI's and the nightly's checkouts and the
README read, where each wrote its own.

**rustc's API**: `Const`'s methods are `ConstExt`'s; `caller_bounds()` is
an iterator, and `ParamEnv::new` takes the `TyCtxt`; THIR's `Use` is
`ValueExpr`; a trait's and a projection's predicates are `TraitClause` and
`ProjectionClause`; the resolver's data is `middle::resolve`'s; `CrateType`
is `rustc_structures`'; `Diag::emit` gives nothing, `emit_err` the
`ErrorGuaranteed`; `IdentIsRaw` is `IdentKind`; a stable hash's def path
hash is a `Fingerprint`; `DefKind::AssocConst` has no fields, its type
const asked of `is_direct_const`; an alias const's `Inherent` is
`InherentImpl`, an impl's args, or `InherentSelf`, of a `Self` type, which
only unstable const generics write, refused.

**A `use` list is one item.** `use a::{b, c::*};` was an item for each
name, its path whole, and a list's stem; it's one item now, its tree
nesting the names, each with a `DefId` of its own and a path relative to
its list's. `src/lower/uses.rs` is the one owner of reading them: each
leaf with its path whole, a list's `self` its list's path, as rustc names
it. Naming, re-exports, held bindings, default exports and the paths a
library's item is named through (`library_paths`) read leaves; a list's
prefix's last segment, where it has no resolution of its own, takes the
list's, and a leaf's span is its tree's, as 1.99's item of it was, so what
re-exports a list writes is as before.

**Each new variant is named, none by a wildcard.** Every public enum of
rustc's MIR, THIR, HIR and type crates was diffed from 1.99 to 1.100.
`ProjectionElem::PhantomDeref`, a `Reborrow` type's phantom dereference,
which only the unstable `Reborrow` traits make, is refused where MIR is
lowered, naming #1653, and the cell analysis takes it as the same place.
The rest are unstable syntax's alone.

**The crate's hash is its HIR's.** 1.100's rustc hashes a crate from its
encoded metadata, and panics where `crate_hash(LOCAL_CRATE)` is asked
before it's encoded, as a library's contract asks it, as it's lowered.
rust-js's session sets `-Z metadata-crate-hash=no`, rustc's own fallback,
the HIR hash 1.99 made. It's sound: the hash the contract names is the one
the library's metadata keeps, which a consumer reads and compares (ADR
0100). A stable toolchain takes `-Z` options from rust-js because its
driver sets them in the session's options itself, as it sets `ub_checks`;
no command line is parsed. The manifest test "a library's manifest names
its crate's hash" fails, a panic, if the option stops being honoured, and
the mutation `crate-hash-from-metadata` holds it.

**std's bodies changed, and so did what rust-js reads of them.**

- `Ord::min` and `max` of an integer call `intrinsics::integer_min` and
  `integer_max`, read as `a.min(b)` is (`Std::MaxOf`).
- `NonNull` is a lang item, no longer a diagnostic item, its field a
  pattern type, `pattern_type!(*const T is !null)`.
- `Infallible` is `!`: `GenericShunt`'s residual of a `collect()` into a
  `Result` is `Result<!, E>`, and `!` is never shown, as an enum with no
  variants isn't (ADR 0161).
- A derived `Hash` binds no `__self_discr`: the discriminant is read by its
  block's call, `write_isize(state, discr)`. An optimized body's
  discriminant read that nothing of its block reads was dropped as the
  read of a switch that went; its block's terminator counts as a read now.
  Without it, std's compiled `IpAddr` and `SocketAddr` hashes read a name
  nothing declares, a `ReferenceError`: a wrong answer the upgrade would
  have caused, which the std package's regeneration, compared with the
  committed one, caught before it shipped.

**std's behaviour changed, so rust-js's did** (results change, Rust's kept,
ADR 0262):

- **`Iterator::max` and `min` are `reduce(Ord::max)`**, the item's own
  `max` where it writes one, else std's `other < self`, by `PartialOrd`,
  where 1.99's were `max_by(Ord::cmp)`. Of items whose order is seen,
  isn't total or is written by hand anywhere in them, whose `<` may not
  be their `cmp`'s, or that write their own `max`, `$reduceBy(items, max)`;
  of the rest, whose `cmp` gives the same one, `$maxBy(items, cmp)` as
  before. `$reduceBy` drops the one kept only where a stage panics inside
  its fold: once folded it's given back, so a panic as what it stepped is
  closed leaks it, #1655's rule for `max_by`
  (`reduce_leaks_what_it_kept_as_its_iterators_drop_panics.rs`). The
  hand-written case is a wrong answer the first cut of this upgrade gave:
  `partial_ord_apart_from_ord.rs` kept the wrong item.
- **A `str`'s `escape_debug()` escapes every Grapheme_Extend `char`**, as
  a `char`'s does; 1.99 escaped the first alone. `$escapeDebug` takes no
  flag.
- **`OpenOptions` that append and truncate** fail with "append and
  truncate cannot both be enabled", where 1.99 said "creating or truncating
  a file requires write or append access".
- **A `String`'s `retain` whose closure panics before it removed any
  `char`** leaves the text whole: 1.100's asks each in place till one goes,
  and only then holds the guard that leaves what it kept. The iterator
  matrix's `string retain | panics` found it, a wrong answer the upgrade
  would otherwise have shipped.

**std's new items are refused, naming their tracker.** A call of an item
1.100 stabilized that rust-js doesn't lower yet, `Vec::from_fn` or
`Rc::map`, is refused at compile time as any unrecognised call is, and its
refusal names #1656, by the item's own `#[stable(since = "1.100.0")]`
(`open_item`, `OpenCall::NewIn1_100`); the std net leaves them out on
that issue. So does an impl of a trait 1.100 stabilized, `Allocator`,
refused as any std trait rust-js doesn't implement is: rustc's
`drop/box-conditional-drop-allocator.rs`, out of scope at 1.99 as its
`allocator_api` was unstable, is a known failure at 1.100 for it, and
`regions/regions-mock-codegen.rs` for calling `Allocator::allocate`. A
value of a `dyn` of such a trait, `dyn Allocator`, refused as a value of a
type rust-js doesn't hold, names it too (`open_type`), as rustc's
`allocator/dyn-compatible.rs` has one; it named no issue when the lists
were blessed at 1.100 (rustc tests run 43391474675).

**Found by the corpus at 1.100.0**, besides those: a `TypeId`'s hash is
1.100's, and std's `FromIterator::from_iter` names its parameter `I`, so a
dictionary's entry's given facts are `IIntoIterTrustedLen` and `dropI`.
The never-type fallback, `()` to `!` in editions 2015 to 2021, changed no
case's answer: the corpus runs against native Rust at 1.100.0.

**What `--test` and Cargo do.** A test's `#[rustc_test_marker]` item is a
`static` since 1.100, a `const` before: it's left out as the `const` was,
as the JS runner takes its place (ADR 0026). Cargo 1.100 keeps each build
step's artifacts in a directory of its own, `<profile>/build/<package>/
<hash>/out`, where 1.99's were all in `<profile>/deps`. rust-js still
writes each crate's JS beside the others', `<profile>/rust-js/<crate>`, so
one imports another's by a short path, `../shared-1a2b/flags.js`, whose
import the formatter lays out on one line: `js_root` in `src/cargo.rs` is
the one place the profile directory is found of a crate's `--out-dir`.
What reads the JS reads where it is of the manifest beside each crate's
metadata, which Cargo's JSON messages name; the in-source ledger is
`ledgerOf`'s, of `cargo metadata`'s target directory; `serde/build.sh`
reads each crate's file of Cargo's messages, where its `ls -t` of `deps`
picked a 1.99 build's rlib; and the tests read the layout through
`cargoOutDirs`, `cargoArtifacts`, `buildScriptOutputs` (a build script's
`run/stdout`, its `output` before) and `rustJsUnits`. Cargo's
unused-dependency lint, on by default, is allowed in `serde/`'s crate,
which is its dependencies alone, and a clippy fixture depends on React
only where it uses it.

Each fix has a mutation: `crate-hash-from-metadata`,
`integer-min-max-intrinsics-unread`, `discriminant-read-by-a-call-dropped`,
`non-null-by-diagnostic-item`, `infallible-an-enum`,
`never-shown-of-enums-alone`, `str-escape-debug-extenders-first-only`,
`append-truncate-in-old-words`, `iterator-max-by-cmp`, `list-names-unread`,
`list-self-named-self`, `crate-js-in-each-out-dir`,
`iterator-max-by-cmp-of-hand-written-orders`,
`retain-panic-before-a-removal-truncates`,
`reduce-by-drops-what-it-gave-back` and `new-in-1-100-unnamed`. Twenty
mutations of the code those changes moved were retargeted at its new form;
`reexports-alone-list-stem-own` is gone, as a list has no stem item of its
own to count since 1.100, its leaves read through `uses`.
