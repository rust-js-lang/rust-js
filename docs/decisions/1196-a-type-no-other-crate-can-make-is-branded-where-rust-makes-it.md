# 1196. A type no other crate can make is branded where Rust makes it

Status: Accepted (2026-11-24, #2403 step 2c; the lead's choices: brand at
construction, not at exits; the runtime brands what the declarations
brand). Extends [1187](1187-a-tuple-struct-no-caller-can-make-is-declared-opaque.md),
whose brand was the declarations' alone, and
[1194](1194-a-value-that-holds-others-is-checked-whole.md), whose checker
of such a type now tests it.

Case ([0262](0262-when-rust-and-js-disagree.md)): **C**, as 1192's.

## Context

A `pub` struct with a field that isn't `pub` is how Rust keeps an
invariant: only its crate's code makes an `Email`, so every `Email` passed
its check. ADR 1187 declares one opaque, so TypeScript takes one only of
what rust-js's JS gave. JS without TypeScript, or with `as unknown as
Email`, still gave Rust `["not an email"]`, which ADR 1194's checker took:
its fields were of their types. A field check can't hold an invariant:
chapter 15's `Temperature { celsius: -500 }` passes it.

## Decision

**What the declarations brand, `sealed` (ADR 1187, `declarations::contract`
its one answer), whose JS is an object of its own, is branded where Rust
makes one, and checked by its brand where JS gives one:**

```js
export function email(text) {
  return $brand("app#7e6d147c3a102184::Email", [text]);
}

function checkTemperature(value) {
  $checkBrand(value, undefined, undefined, "app#7e6d147c3a102184::Temperature", "a Temperature");
  $checkObject(value);
  $checkField(value, "celsius", $checkF64);
}
```

- **A brand is a `WeakSet` of the objects Rust made of the type** (O(1)
  to test, nothing JS can forge), one per type, not per instantiation:
  `Id<String>` and `Id<u32>` are one `Id` set, as the declarations' one
  `$idBrand`.
- **Found by the type's key in a registry every copy of the runtime
  shares**, `globalThis[Symbol.for(..)]` (ADR 1184, #2333): its crate's
  name, its disambiguator and its path, `app#7e6d…::Email`. A library
  bundled with its own runtime and the app beside it read one set, so a
  value one copy branded the other takes. Two versions of one crate have
  two disambiguators, two types to Rust, so two sets.
- **Its fields are still checked** after its brand, as JS can write into
  an object Rust made, `email[0] = 5`.
- **Branded where each maker makes one** (`boundary/brands.rs`'
  `branded`, called by each):
  - `adt_value`, every literal and constructor call, MIR's and THIR's;
  - a constructor as a value, `.map(Email)` (`constructor_value`);
  - a copy, `{ ...p }` (`copy_parts`), a `Copy` type's move and ADR
    1197's copies among them;
  - a clone (`clone_parts`);
  - a derived `Default`;
  - serde's reader (`serde/de.rs`' `construct`);
  - a `const`: its initializer, lowered as a body is, never rustc's value
    of it, which would be unbranded (`representation/consts.rs`).
  - A struct update, `{ ...base, x }`, is `adt_value`'s.
- **Not branded:**
  - **A `#[repr(transparent)]` newtype of a number or a string,** whose JS
    is its field: no `WeakSet` holds a primitive. It keeps its field
    check and the declarations' brand. (So react-in-rust's most made
    sealed type, `EventPriority` of a `Lane`, pays nothing.)
  - **One that always holds another** (`always_holds`), whose object is
    its field's: branding it would brand the field's value.
  - **std's,** `Ipv4Addr` or an iterator's adapter, whose declarations
    are std's own (ADR 1149), not `sealed`'s, which declares a crate's.
  - **A library's type, checked by its consumer**: a consumer's function
    of one checks it as any library type, not yet; the library's own
    entry checks it, by the one registry.

## Cost

- `WeakSet.add` is ~25–30 ns a construction, against 1.5–9 ns to make the
  object; the key's lookup adds ~7–10 ns; `has` is ~1 ns. Every Rust
  construction of a branded type pays it, as a brand only Rust's makers
  give is the point.
- React's port makes 14 sealed types at 30 sites; the hottest it brands
  is the hooks' `Effect`, one an effect a render.
- A module-level set, read once, would take the lookup off each
  construction: not yet, as its declaration's plumbing outweighs 8 ns.

## Why

- **Rust never reads a value of a type no other crate can make that its
  crate didn't make.** A JS caller can't forge an `Email`, nor a
  `Temperature` of -500.
- **At construction, not at exits:** a missed exit refuses a value Rust
  made, which is worse than a forgery missed. Each maker is listed above,
  and the guard test holds each.
- **One predicate:** the runtime brands exactly what the declarations
  brand, by `sealed`, so the two can't drift.

## What holds it

- `test/boundary-brands.test.ts`, the guard: JS's own refused; each maker
  above, given to JS each way a value leaves (a return, in a `Vec`, an
  `Option`, an `Rc`, a callback's argument, a callback's result) and given
  back; a generic's one set; a primitive newtype's field check; a library's
  type its consumer clones on another copy of the runtime; two versions of
  a crate, two keys; serde's reader.
- A mutation of each maker's brand, of the checker's, and of the shared
  registry, `brands-per-copy`, each caught.

## Amendment: a sealed type's constant is rustc's value, branded (#2403, 2026-11-24)

A constant of a type no other crate can make is rustc's value of it with its brand's mark on its literal,
`$brand("..::NaiveDate", { yof: 49152 })`, not its initializer lowered where it's used: chrono's
`NaiveDate::AFTER_MAX` used in another module read `date.rs`'s private `MAX_YEAR` there, and bitflags'
`Perms::READ` called `Perms` inside its own literal. A module's constant keeps the mark through
`aggregates::const_object`'s layout. The crate corpus's snapshots keep a brand's key without its crate's
hash, `itertools#<hash>::..`, which Cargo's build metadata moves with each compiler.

## Amendment: a value of no size is branded as its constant is (#2403, 2026-11-26)

A value of a sealed type of no size that a consumer's copy of a library's generic code makes, where its
optimized MIR wrote none (`_0 = Checked(())` gone, #1889), is its constant's: `zero_sized_js` gives it
through `const_js`, which brands it as above, `$brand("..::Checked", [undefined])`. `zero_sized_value`
had a branch of its own that made it of its fields' values and branded it again: no test, probe or
corpus case, nor itertools' crate, told the two apart, the branch's mutant giving the same JS, so it's
gone, one owner of a sealed constant's brand. `--retire`'s run couldn't judge it: its port control
refused, React's port at its pin regenerating otherwise with #2403 itself. The guard test gains the
case, a proof token `check<N: AsRef<str>>(name) -> Checked` given a name that drops, so the consumer
compiles its own `check`.

A brand on a body's literal is its mark or the call around it, `$brand(key, { .. })`, printed alike:
no test or probe told them apart either (`unmarked-brand`'s mutant): a guard's literal, which tidy
elides, is of a value that never leaves Rust and so is never branded (`unreached-value-branded`). The
mark stays, one form with a constant's, which `aggregates::const_object` reads; the mutation of an
object literal's mark is now its brand left off, `object-literal-unbranded`, which the guard test's
literal catches.

## Amendment: a brand's key names its crate wherever its files are (#2403, Tu's decision, 2026-11-27)

A brand's key was its crate's name and rustc's `StableCrateId`, which
rustc makes of Cargo's `-C metadata`, and Cargo hashes a path dependency
outside the workspace by its directory. React's port's bindings are
`../rust-js`, beside wherever the port is checked out, so the same
compiler and the same port commit, built in two directories, branded each
type by two keys (`scheduler#15eac83c8fb08eb1::Task` and
`scheduler#a55a7b5f862097d6::Task`): a pinned port would have differed in
every other checkout, CI's nightly too. **Pins and snapshots made before
this amendment may carry machine-specific crate hashes**; the port's pin
of #2403 is made after it.

**A crate's identity is its name and a hash of what tells its package
from every other a program may hold, of no path**
(`rust_js_program::library::crate_identity`, FNV-1a of each part's length
and bytes, so no two lists of parts hash as one): whether it's a program
or a library, as rustc's own tells a binary from a library of one name;
and, of Cargo's build, its package's name, version and source
(`cargo::package`): a registry's index directory, a git checkout's
repository, commit and directory in it, as Cargo's home lays them out,
`vendored` of a directory source, or `path`, as Cargo refuses two path
packages of one name and version in a build. Of a compile outside
Cargo's, rustc's `-C metadata` values, sorted, which tell two crates of
one name apart there as Cargo's don't. A crate writes its identity in its
library's contract, `Library::crate_identity` (library ABI 50), and a
consumer reads a library's there (`recognition::crate_identity`), never
making its own. A crate rust-js didn't compile, one rustc alone built whose
generic code its consumers compile (indexmap's, serde's, the crates
`cargo.rs`' `READ_AS_RUSTC_BUILDS_THEM` names), writes no contract: it's
identified by rustc's `StableCrateId`, which Cargo makes of no path of a
registry's crate, as those are; one patched to a path outside the
workspace would carry its directory (#2616). The crate corpus's indexmap
row holds it, and its probe: its `IndexMap`, branded where the probe
makes one, crashed the compile when only a contract named an identity
(test/bound-maps.test.ts' "indexmap's maps run as std's", mutation
`rustc-alone-crate-unidentified`).

**As unique as rustc's: two crates a program would hold of one identity
are refused**, naming both, with what to write (depend on one, or give
one another version, or another `-C metadata`)
(`recognition::refuse_shared_identities`): their types' brands would be
one, and one's value would pass for the other's. Cargo gives no two
crates of a build one, nor rustc two of one `-C metadata`; two vendored
copies of one package could.

Not changed, and listed: rustc's `StableCrateId` still keys a library's
contract within a build (`Library::key`), which no output carries; a
`TypeId`'s text (`std_types/any.rs`) is still of it and of rustc's type
hash, and a Cargo build's import of a library's JS names Cargo's
directory of it, `sealed-<-C metadata>/lib.js`: both machine-specific
where a path dependency is outside the workspace (#2616); React's port's
`rust-js build` names its packages, so its JS is the same in every
directory.

Held by `test/cargo-workspace.test.ts`' "a brand's key is the same of
every build of rust-js and every directory, and two versions' are two"
(two path dependencies outside the workspace, renamed, of two versions;
the same files in two directories give each library the same JS), and
`test/boundary-brands.test.ts`' "a program of two crates of one identity
is refused, naming both"; unit tests `identities_tell_apart_what_rust_does`
and `a_packages_source_is_of_no_path`. React's port regenerated in two
temporary directories is byte for byte the same. Mutations
`package-identity-of-its-directory`, `package-identity-versionless`,
`package-source-registry-unnamed`, `package-source-git-commit-dropped`
(`cargo.ts`), `identity-blind-to-programs`, `identity-parts-joined`
(`library.ts`), `library-identity-of-its-own`,
`shared-identities-unrefused` (`lower/recognition/std_crates.ts`). Every
snapshot's key moves once, a hash for a hash.
