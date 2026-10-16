# Changelog

Each release of rust-js's compiler and the packages released with it, at its
version: `@rust-js/native`, `@rust-js/build`, `@rust-js/typescript`,
`@rust-js/runtime`, `@rust-js/create`, `@rust-js/react` and `@rust-js/next`
([ADR 0120](docs/decisions/0120-first-npm-release.md)). `@rust-js/builtins`,
`@rust-js/webapi` and `@rust-js/node` are released on their own, each at its
version ([ADR 0116](docs/decisions/0116-binding-versions.md)).

## 0.0.4

For macOS on Apple silicon, with Rust 1.99.0, from 1.98.1. `@rust-js/builtins`
is 0.0.2 and `@rust-js/webapi` 0.0.3, each its own release (ADR 0116). New
with it: `@rust-js/typescript`, which prints each module's `.d.ts`;
`@rust-js/next`, Next.js's crate, at 0.0.4; and `@rust-js/node`, Node.js's,
at its own 0.0.1. `@rust-js/vite-plugin` and `@rust-js/resources` aren't
released any more: `rust-js build` is the one way to build an app.

### New

- **`rust-js build` is how an app's Rust is compiled**, a command of its
  own beside `vite` or `next dev`, run once each time: each module's JS and
  `.d.ts` beside its Rust, to commit, and `rust-js build --check`, an app's
  `build` script's first step, fails where what's committed is stale
  ([ADR 0412](docs/decisions/0412-rust-js-build-is-its-own-process.md)).
- **A Rust library is shared on npm, as ReScript's are**: one package of
  its crate and the JS its author's `rust-js build` wrote. A JS app imports
  that JS; a rust-js app names the crate in its `Cargo.toml` and compiles it
  by its own rust-js into the package, wherever npm, bun or pnpm put it,
  and imports it by the package's name, `@acme/greet/src/lib.js`. A
  package's files are replaced, never written through, so a store sharing
  them keeps its own, and `rust-js build --check` is of the app's JS alone,
  a failed build of the app's errors, not a package's warnings. A binding
  crate says it's one, `[package.metadata.rust-js] bindings = true`, and is
  rustc's to check
  ([ADR 0414](docs/decisions/0414-a-library-on-npm-as-rescripts-are.md)).
- **Next.js**: an app's routes and components in Rust, beside its JS, one
  file at a time; the next crate binds all of Next.js, `next/link`,
  `next/router`, `next/image`, `next/font`'s Google Fonts, metadata,
  `getStaticProps` and the rest, typed as Next.js types them, and an app's
  `next.config` may be Rust (ADRs 0192, 0346, 0350, 0354, 0359, 0378).
- **TypeScript declarations**: a `.d.ts` beside each module's JS, printed by
  TypeScript, so the TypeScript of an app moving to Rust is checked against
  it, under any `tsconfig.json` (ADRs 0196, 0207, 0413).
- **The bindings cover what TypeScript types**: `webapi` TypeScript's DOM,
  `js` its ES library, `react` @types/react, `node` @types/node, each
  measured against them, and coverage only grows (ADRs 0281, 0283, 0347,
  0351).
- **rustc's own tests that rust-js compiles and runs as Rust does: 2,062 of
  2,214**, from 1,899 of 2,206. Bodies are lowered from rustc's MIR (ADR
  0364). Among what compiles now: unions; raw pointers, a handle on their
  place; plain data as its bytes, `transmute` and `zeroed` of it; `i128`
  and `u128`; `Duration`; `Wrapping` and `NonZero`; every std data
  structure, or an error that names it, `LinkedList`, `BTreeMap`'s entries,
  `Cow`, `Rc`'s counts, `RefCell`'s borrows; an iterator of the crate's
  given to generic code; a struct of a `dyn` last field; functions that
  never return.
- **JS as a person writes it**: destructuring, `const`, labeled blocks,
  loops, spreads and calls, conditionals for two-arm `match`es, imports in
  one block, laid out by the app's Prettier, a file's leading comments, a
  license's, kept (ADRs 0291, 0278, 0313, 0381, 0383).

- **A `NonNull<T>` is its raw pointer**: `new`, `as_ptr`, `as_ref`,
  `as_mut`, `cast`, `from(&mut x)` and the rest, as a raw pointer's; and
  an array's items are reached as arrayvec reaches them: casts where JS has
  the same place, `ptr::copy`, `slice::from_raw_parts`, and a generic
  constant, `[const { MaybeUninit::uninit() }; CAP]`, as code. A pointer to
  a slice, `*mut [T]` or `NonNull<[T]>`, is the slice; a `&mut` slice's
  `as_mut_ptr()` is no longer refused; and a pointer only read through,
  arrayvec's `ptr::read(elt as *const _)`, is a cell of a value with no
  place
  ([ADR 0407](docs/decisions/0407-a-raw-pointer-is-a-handle-on-its-place.md)).
- **A writer of bytes of the crate's, `impl io::Write`**, where a `W: Write`
  goes or of its own type: its `write_all` and `write_fmt` std's, its
  `write` till all's written (ADR 0392).
- **arrayvec 0.7 runs as natively**, `ArrayVec` and `ArrayString` alike
  (`docs/crate-corpus.md`): a crate built on raw pointers into an array of
  `MaybeUninit`s.
- **`size_hint()` of a generic `I::IntoIter`**, as a type parameter's is
  (ADR 0170).
- **`[const { MaybeUninit::uninit() }; N]` of a type that isn't `Copy`**:
  `Array.from({ length: N })`, a constant with no byte written being
  `undefined` where it's used (ADRs 0127, 0332).
- **smallvec 1.16 runs as natively** (`docs/crate-corpus.md`): its
  `SmallVec` inline and spilled to the heap, and what it drops, and when.
- **`iter::repeat(value)` of a value with a destructor**, and `take` and
  `skip` of it: `value` is dropped when the iterator is closed, JS's
  `return()`; and `v.extend(items)` of items with a destructor
  ([ADR 0422](docs/decisions/0422-an-iterator-holding-a-value-is-closed-to-drop-it.md)).
- **Generic code given a `T` with a destructor** that swaps its items, lends
  one, or extends from a slice, or `set_len`s its vector: it drops none,
  so it takes one (ADR 0422).
- **`MaybeUninit::<A>::uninit()` of a type parameter that's an array**,
  smallvec's `A: Array`: an array of slots, written an item at a time
  (ADR 0417).
- **A std trait's associated type dropped in generic code**, an
  `I: IntoIterator`'s `I::IntoIter`, an `I::Item`: its caller gives its
  drop, `dropIIntoIter`, as it gives a type parameter's
  ([ADR 0421](docs/decisions/0421-std-associated-type-drops-are-given.md)).
- **Fixed: an iterator of the crate's with a `Drop`, given where a generic
  iterator goes**, was dropped as the JS iterator made of it, which its
  `drop` read as its own (ADR 0421).
- **`Vec::set_len`**: what it shortens a vector by stays where it was, read
  through a pointer as smallvec's `from_vec` does, and what it lengthens it
  over is what was written there
  ([ADR 0420](docs/decisions/0420-what-set-len-leaves-stays.md)).
- **A `Vec`'s `capacity()`**, kept where the crate or a library it uses
  asks it, as std promises it
  ([ADR 0419](docs/decisions/0419-a-vecs-capacity-kept-where-asked.md)).
- **`v[i]` of an index of a type parameter's**, `I: SliceIndex<[T]>`, an
  `Index<I>` of the crate's: its dictionary's, the runtime's for `usize` and
  each kind of range
  ([ADR 0418](docs/decisions/0418-a-generic-slice-index-is-a-dictionary.md)).
- **An iterator whose `Item` is another trait's associated type**,
  `A::Item`, looped over by `&mut` (ADR 0364).
- **`==` of two types' items**, `[A::Item] == [B::Item]` by `A::Item:
  PartialEq<B::Item>` (ADR 0053), and `clone_from_slice` of an associated
  type's items (ADR 0324).
- **A `MaybeUninit`'s `as_ptr()` and `as_mut_ptr()`**, of a type parameter's
  too (ADR 0332).
- **The allocator**: `alloc`, `realloc`, `dealloc`, `handle_alloc_error`,
  `Layout`, a pointer cast to bytes and back, and `Vec::from_raw_parts`. An
  allocation is a JS array of slots
  ([ADR 0417](docs/decisions/0417-an-allocation-is-an-array-of-slots.md)).
- **`size_of::<A::Item>()` in generic code**, of an associated type of the
  crate's trait: its dictionary's, `AArray.ItemSize` (ADR 0145).
- **`p.offset_from(q)`** of two pointers into one array (ADR 0407), and
  **`next_power_of_two()`** and its checked one (ADR 0064): hashbrown's.
- **An iterator of the crate's with a destructor, consumed**, `collect()`,
  dropped after, as Rust's method drops it (ADR 0098); and
  `ptr::drop_in_place` of a slice drops each item (ADR 0407).
- **A `str`'s `as_ptr()`, only read through**, a copy's source: a pointer to
  its UTF-8 bytes, as arrayvec's `ArrayString` copies them in (ADR 0407).
- **`as_ref()` of text as a `Path`**, `s.as_str().as_ref()` in an
  `AsRef<Path>` impl: the text itself (ADR 0173).
- **A zeroed `[MaybeUninit<u8>; CAP]`**, of a length generic code is given,
  `new Array(CAP).fill(0)` (ADR 0332).
- **`map_err` of a `fmt::Result`**, `write_fmt(args).map_err(..)?` of a
  writer that may fail (ADR 0187).
- **A trait's default method calling its `Self`'s**, where `Self` is
  generic, `Holder<T, CAP>`: given the impl's `CAP`, and another default
  of it found in the impl's dictionary (ADR 0135).
- **`clone_from_slice` in generic code** calls each item's own
  `clone_from`, where its impl writes one, through its `Clone` dictionary,
  as std's does (ADR 0324).
- **A `&mut &mut u32`**, a `&mut` to a `&mut` of a number or a string, is a
  cell of the cell, as arrayvec's `ScopeExitGuard` holds one
  ([ADR 0338](docs/decisions/0338-slice-split-off.md)).
- **A generic function or impl that asks `size_of::<T>()` is given it** in a
  trait impl's method, through the impl's dictionary, called through its
  trait from another impl's, or given that dictionary for a bound or
  `collect()`, and across crates, by
  its library's manifest ([ADR 0145](docs/decisions/0145-type-facts.md)).
- **A range's bounds in generic code**, `R: RangeBounds<usize>`'s
  `start_bound()` and `end_bound()`, of every range of std's
  ([ADR 0416](docs/decisions/0416-range-bounds-in-generic-code.md)).
- **A crate of crates.io's is compiled into the app**, where rust-js can
  compile it: its JS committed in `rust-js/crates/<name>-<version>/`,
  imported from there, and checked by `rust-js build --check`. One it
  can't is its types alone, as before, and a call into it says why
  ([ADR 0415](docs/decisions/0415-a-crate-of-crates-io-in-an-app.md)).

### Changed

- **`@rust-js/vite-plugin`, its `rust-js:<package>` imports, `rust-js-patch`
  and its `postinstall`** are gone: `rust-js build` writes the patch Cargo
  reads, and an app imports the JS beside its Rust (ADR 0412).
- **A module's header names its Rust from its crate's directory**,
  `src/api.rs`, whichever build wrote it, not `frontend/src/api.rs` from
  the workspace's (ADR 0414).
- **`react::Ref<T>` is @types/react's `Ref<T>`**, a ref a prop holds, an
  object or a callback, and `react::DetailedHTMLProps<E, T>` and
  `react::SVGProps<T>` an element's props, its `ref` too, which a
  component's props flatten. The trait of what JSX's `ref` takes is
  `IntoRef` ([ADR 0384](docs/decisions/0384-a-ref-prop-is-reacts-ref.md)).
- **`[package.metadata.rust-js.format]` is gone**: the app's Prettier lays
  out what rust-js writes; move its options to `.prettierrc`
  ([ADR 0381](docs/decisions/0381-the-apps-prettier-lays-it-out.md)).

### Fixed

- **rustc no longer crashes** where a generic function calls another whose
  parameter is a projection, `<T as Deref>::Target`: whether the callee's
  value is boxed is asked in the callee's environment, not its caller's.

- **A declaration means the same under every `tsconfig.json`.** A field a
  caller may leave out is `?: T | undefined`, as @types/react writes one,
  where `?: T` refused `undefined` under `exactOptionalPropertyTypes`,
  which Rust's `Option` takes as `None`. The declarations are checked under
  `bundler`, `nodenext` and the strictest settings, Vite's template's
  `erasableSyntaxOnly` among them
  ([ADR 0413](docs/decisions/0413-a-declaration-means-the-same-under-every-tsconfig.md)).
- **A `match` of a call's value calls it once.** With three arms or more,
  `match note(x) { 1 => .., 2 => .., _ => .. }` called `note` once per arm
  it tested. Found by a generated program
  ([ADR 0364](docs/decisions/0364-mir-lowering.md)).
- **A loop left by a `break` of a choice compiles.** `loop { if a || b {
  break if c { 1 } else { 2 }; } }` never finished compiling: rust-js turned
  its two exits around, and back, without end. Found by a generated program
  ([ADR 0370](docs/decisions/0370-loop-exits-first.md)).
- **A private props struct an exported component names is declared in its
  `.d.ts`**, not exported, where it was `any`
  ([ADR 0385](docs/decisions/0385-a-private-type-a-declaration-names.md)).
- **A crate root of `mod`s alone writes no JS**, an app's list of its files;
  and a module of a `js::import!` alone writes its import, which was lost
  ([ADR 0382](docs/decisions/0382-a-module-has-a-file-for-what-it-has.md)).
- **A `&mut` of what a generic `Option`'s `Some` holds compiles**, `Some(x)`
  of a `&mut Option<T>`, read, written or read in a guard, as chrono's
  `set_if_consistent` does; **another crate's generic writer is given a
  `Formatter`**, bitflags' `to_writer`; and **an enum of no variants is
  declared `never`**, where its `.d.ts` didn't parse. chrono compiles and
  bitflags runs again ([ADRs 0051](docs/decisions/0051-generic-options.md),
  [0180](docs/decisions/0180-generic-writers.md)).
- **Switching an app's rust-js, up or down, builds its crates again.** Cargo
  had the crates another rust-js built as done, as an installed compiler's
  file is as old as its package says, and the build stopped at
  `incompatible dependency compiler identity`. Now `rustc -vV`, through
  rust-js, says which rust-js it is, and Cargo keeps each one's build apart
  ([ADR 0101](docs/decisions/0101-cargo-workspace-wrapper.md)).

### May now be refused

What 0.0.3 compiled wrongly, or not at all, is now right or an error that
names it:

- Memory addressed by number, a pointer cast to an integer or read at an
  offset, is an error that says so (ADR 0390).
- A crate's `[package.metadata.rust-js]` `declarations` key or `format`
  table is an error that says what replaces it.
- An npm package with a crate installed under an alias of another name,
  `"x": "npm:@acme/greet"`, or whose `exports` hide the JS an app imports,
  is an error that says what to do (ADR 0414).

### Upgrade from 0.0.3

1. Install Rust 1.99.0, which rust-js 0.0.4 runs with:
   `rustup toolchain install 1.99.0 --profile minimal --target wasm32-unknown-unknown`.
2. In `package.json`, set `@rust-js/runtime` and `@rust-js/react` to `0.0.4`
   in `dependencies`, `@rust-js/builtins` to `0.0.2` and `@rust-js/webapi`
   to `0.0.3`, and `@rust-js/native` and `@rust-js/build` to `0.0.4` in
   `devDependencies`. Remove `@rust-js/vite-plugin`, and the `postinstall`,
   `rust-js-patch`. Make the `build` script `rust-js build --check && vite
   build`.
3. In `vite.config.js`, remove `rustJs(..)` and its import: Vite serves the
   JS beside your Rust as any JS.
4. In `Cargo.toml`, name the crates' releases:
   `react = { package = "rust-js-react", version = "~0.0.4" }`, and
   `~0.0.2` and `~0.0.3` for `js` and `webapi`. Remove a `declarations`
   key: a `.d.ts` is always written.
5. `bun install` (or npm's, pnpm's), then `bunx rust-js build`: the JS and
   `.d.ts` beside your Rust are 0.0.4's. Commit them with the upgrade.
6. From now on, run `rust-js build` after each change to your Rust, beside
   `bun run dev`.

### Roll back to 0.0.3

Revert the upgrade's commit, its `package.json`, `vite.config.js`,
`Cargo.toml` and the JS, then `bun install`, and `cargo clean --target
wasm32-unknown-unknown`: Cargo keeps each rust-js's build apart from 0.0.4
on, but not 0.0.3's.

## 0.0.3

For macOS on Apple silicon, with Rust 1.98.1, as 0.0.2. `@rust-js/builtins`
stays 0.0.1 and `@rust-js/webapi` 0.0.2.

### New

rustc's own tests that rust-js compiles and runs as Rust does: 1,899 of
2,206, from 1,583. Among what compiles now:

- **Text by its bytes:** a string's `len()`, slices and `find` count UTF-8
  bytes, as Rust's do; `s.bytes()`, `as_bytes()` and byte strings.
- **Numbers:** `f32`, each result rounded as Rust rounds it; bitwise
  operators in generic code and a type's own, as a set of flags has them.
- **Patterns:** slice and array patterns, bindings inside `|`, `if let` as a
  value anywhere, if-let guards, inline `const` blocks.
- **Formatting:** `{:#?}` and the Debug builders; a placeholder's width,
  precision and sign reach a generic or `dyn` value's `fmt`, which can ask
  its `Formatter` for them.
- **Traits and generics:** const generics of traits, a `dyn Display` or
  `dyn Error` as a value, `Box<dyn Error>` from `?`, generic associated
  types of lifetimes, `size_of` and `type_name` of a type parameter.
- **Iterators:** std's sources, lazy `chain`, `zip`, `take_while`,
  `skip_while`, `fuse`; ranges as values; iterator trait objects; a generic
  iterator stepped through or lent as a `&mut`; an iterator's `len()`;
  `collect()` into a `Result` or an `Option`.
- **Shared state:** `Mutex`, `RwLock` and `Arc` on one thread, channels on
  one thread, a `HashMap` or `HashSet` keyed by a struct.
- **Destructors** run where Rust runs them for closures, loops, trait
  objects and temporaries too.

### Fixed: programs that gave a wrong answer

Each of these compiled in 0.0.2 and ran differently than Rust. A program that
did any of them prints Rust's answer now:

- **A `&mut` of a local a function keeps in another parameter**,
  `keep<'a>(v: &mut Vec<&'a mut u32>, x: &'a mut u32)`, or in `self`, is a
  handle on the local: a box of it was copied back when the call returned,
  and the writes through it after were lost (ADR 0099).
- **A reborrow of a reference reassigned before it's used**, `add(&mut *x,
  { x = &mut q; 10 })`, wrote what `x` referred to after, not before
  (ADR 0099).
- `collect::<Result<Vec<_>, _>>()` and `collect::<Option<Vec<_>>>()` gave the
  array of `Result`s or `Option`s, not the first `Err` or `None`.
- `println!("{} {:?}", v.len(), v.pop())` read `v.len()` after the pop.
- `next()` of an iterator chain could take more than one item, and a chain
  whose closures do what can be seen, kept in a variable or given to `zip`
  or `chain`, could run them in another order than Rust's.
- A width or a sign for a generic or `dyn` value was dropped: it reaches the
  value's `fmt` now.

### May now be refused

What 0.0.2 compiled wrongly is now either right or an error that names it.
A program that relied on one of these stops at the error, where it ran
wrongly before: a width or a sign for a `&dyn Debug` made elsewhere, or for
serde_json's types, whose `fmt`s are rust-js's own.

### Upgrade from 0.0.2

1. In `package.json`, set each package released with the compiler to
   `0.0.3`: `@rust-js/runtime` and `@rust-js/react` in `dependencies`,
   `@rust-js/native`, `@rust-js/build` and `@rust-js/vite-plugin` in
   `devDependencies`. Leave `@rust-js/builtins` and `@rust-js/webapi`.
2. In `Cargo.toml`, name the react crate's release:
   `react = { package = "rust-js-react", version = "~0.0.3" }`.
3. `bun install` (or npm's, pnpm's): its `postinstall`, `rust-js-patch`,
   points Cargo at the new crates in `node_modules`.
4. `bun run dev` or `bun run build`: rust-js writes the JS beside your Rust
   again. Commit it with the upgrade: its diff is what the new compiler
   writes differently.

If the build stops at `incompatible dependency compiler identity`, Cargo
kept crates the other rust-js built: `cargo clean --target
wasm32-unknown-unknown`, then build again. 0.0.3 and 0.0.2 don't tell Cargo
which rust-js built a crate; the next release does.

### Roll back to 0.0.2

The same steps, with `0.0.2` and `~0.0.2`: `bun install` again, then
`cargo clean --target wasm32-unknown-unknown`, which this direction needs,
then build, and the JS beside your Rust is 0.0.2's again (`git checkout`
of it does the same).

## 0.0.2

An app is a Cargo package with its crates from npm, `@rust-js/react` among
them, first released here; its patch is written as it's installed and as
Vite starts, and a Cargo build runs the compiler's Rust. A release build
has no path of the building machine's home in it.

## 0.0.1

The first release on npm, for macOS on Apple silicon: `bun create @rust-js`
makes a Vite and React app written in Rust, with `@rust-js/builtins` and
`@rust-js/webapi`.
