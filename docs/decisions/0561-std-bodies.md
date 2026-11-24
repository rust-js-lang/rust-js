# 0561. std's bodies are std's own; a recognizer is a proved rewrite to an idiom

Status: Accepted. Extends [0364](0364-mir-lowering.md),
[0314](0314-std-data-structures.md), [0100](0100-separate-crates.md),
[0185](0185-library-trait-defaults.md) and [0049](0049-traits-and-generics.md);
amends [0023](0023-strings-references-shared-state.md), whose "one known
item at a time, each with its JS meaning" is the primitives' rule alone,
[0006](0006-errors-and-unsupported-features.md), whose "calling `..` yet"
is reached only at a leaf of addressable memory, and
[0071](0071-stepping-iterators.md); supersedes in part
[0514](0514-a-peekable-of-what-has-a-destructor.md) and
[0543](0543-a-held-peekable-keeps-what-it-peeked-at.md), whose `Peekable`
is std's own now.
Amended by [0983](0983-a-fuse-that-drops-its-source-stays.md): a `fuse()`
stepped in place is left out only where it's given no drops. Amended by
[1054](1054-a-crate-is-stds-by-the-lang-item-it-defines.md): std's crates
are known by their lang items, not their names, and `@rust-js/std` is
their library by its package's name, which a program's crate named
`core` can't take.

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a std function's
answer, its order of effects, its drops and its panics are Rust's, as its
body says. N for where the JS of a std body goes.

## Context

A std call was lowered one way: `recognition::classify` said which std
function it was, by its lang item, diagnostic item or path, and its type's
module wrote its JS (ADR 0314). What `classify` didn't know was refused:
`rust-js does not support calling .. yet`. Each addition was an ADR: of the
145 decisions 0400 to 0544, 42 were iterator adapters, each hand-modeled
with its own runtime helper, `Debug`, clone, size hint and direction,
`$fuse` (ADR 0473), `$peekable` (ADR 0543), a held `Enumerate` (ADR 0542).
std's `Peekable` is 300 lines of safe Rust, `{ iter, peeked: Option<Option<I::Item>> }`,
with those derived.

The bodies are there to read. The toolchain rust-js pins ships, in its
wasm32 metadata, the optimized MIR of every std function that's generic
or inlinable, and `rustc_private` reads it: `tcx.is_mir_available`,
`tcx.optimized_mir`. rust-js read the app's own MIR already (ADR 0364); a
library's it never had, as ADR 0100 says, which is why a library is
compiled to JS of its own.

`--std-coverage` measures what those bodies are made of, beside whether
`classify` knows each method (`docs/std-coverage.txt`, held by
`test/std-coverage.test.ts`), each body the worst it reaches: `safe`, MIR
down to the vocabulary rust-js lowers anyway, a trait's method on a type
parameter, a closure's call, `?`, a drop, a panic; `spec`, reaching a
std-private trait's method on a type parameter, one only std can name,
private or unstable; `opaque`, reaching a body with no MIR shipped;
`memory`, reaching a raw pointer made or read through, a transmute, an
intrinsic or `std::ptr`, as `pointers::mir_addresses_memory` reads it;
`none`, no MIR of its own. Of the data structures' 827 methods at
1.99.0: 231 safe, 21 spec, 5 opaque, 543 memory, 27 none. Of the
`Iterator` trait's 60 provided methods: 56 safe, 4 spec. Of the 22
adapters' 190 methods, their impls of the iterator traits and their own:
136 safe, 40 spec, 14 memory, none opaque or without MIR.

The split is by stratum, not by chance:

| Stratum | Bodies | Examples |
|---|---|---|
| Values over the trait vocabulary | safe | `Option` 39 of 46, `Result` 34 of 36, `Pin` 16 of 16, `BTreeMap` 11, a slice's `contains`, `Vec::len`; `Iterator`'s `try_fold`, `fold`, `count`, `position`, `max_by` and 51 more; `Peekable::next` and `next_back`, `Rev`'s 15 of 15, `Flatten`'s 11 of 11, `Map`'s 9 of 10 |
| Over a trait only std names | spec | `str::contains`, `starts_with`, `find` and `strip_prefix` through `Pattern`, unstable; a slice's `contains` through `SliceContains`, its `fill`, `concat` and `join`; `LinkedList::contains`; `Fuse::next` through `FuseImpl`, `Zip`'s 5 through `ZipImpl`, `StepBy`'s 9 through `StepByImpl`, `Iterator::sum` through `Sum` |
| The containers and text | memory | `Vec::push` through `push_mut`'s raw pointer, `dedup_by`'s `copy_nonoverlapping`, a slice's `sort_by` (driftsort), `iter` (`Iter::new`), `str::split`, `trim` and `lines` (a `Pattern`'s searcher over bytes), `String::push_str`, every `Rc` and `Arc`; `Peekable::peek`, `peek_mut` and `size_hint` |
| Non-generic, not inlined | none | `str::to_lowercase`, `Instant`'s and `SystemTime`'s, `String`'s 9 |

The registry's types are the primitives, so most of their methods reach
memory, as they should: a JS `Array`, string and `Map` is the idiom, and
their Rust bodies a SwissTable and a driftsort. The safe stratum is what's
written over them: `Option`, `Result`, the iterator adapters and the
`Iterator` trait's provided methods, `cmp`, `fmt`'s traits. That was the
stratum rust-js modeled by hand, one ADR at a time.

## Decision

**A std body is lowered as any body is, from the MIR the toolchain ships,
into `@rust-js/std`, a library built into the compiler; a recognizer is
tidy's proved rewrite of such a body into the JS idiom; a leaf of
addressable memory is refused as before.** One way a std call is lowered,
with a readability pass, as ADR 0364 has one way a body is lowered, with
tidy's.

```text
a std call
│
├─ classify knows it ──────────► the idiom: v.filter(f), m.get(k), `${a}${b}`
│                                 (a proved rewrite of the body below)
├─ its body is MIR ────────────► @rust-js/std's function, called as a
│   down to the vocabulary        library's is (ADR 0100): dictionaries,
│                                 drops, each std call in it the same way
└─ its body reaches memory ────► refused where it's reached, as before:
                                  "rust-js does not support .. yet"
```

- **`@rust-js/std` is a library of ADR 0100's, whose bodies are std's.**
  `bun run std` compiles `std/lib.rs`, a crate of nothing but the modules
  std's bodies are written into, `iter` for `core::iter`'s, with
  `--std-bodies`: `collect_std_bodies` adds each body
  `recognition::compiled_std` names, `COMPILED_STD` by type, impl and
  method and `COMPILED_STD_PROVIDED` by trait and provided method, with
  the closures each makes, as a `Body` of an external `DefId` with no THIR
  and its `optimized_mir`. Its JS and its manifest, with paths relative
  to it, are committed in `std/`, released as `@rust-js/runtime` is (ADR
  0103), and the manifest is built into the compiler
  (`library::STD_MANIFEST`): every crate but that one is compiled against
  it, `Dependencies::add_std`, its items found by the crate each is of,
  `core`'s, and imported by the package's name, `@rust-js/std/iter.js`.
  `test/std-package.test.ts` holds the committed package equal to what
  the compiler writes.
- **A std body is lowered as the crate's own are**, by the same `FnCx`,
  with an empty THIR as a dictionary has (`pipeline.rs`'s `no_body`),
  its names from MIR's `var_debug_info`; its module `body_module`'s, the
  crate's module named as std's is, `recognition::std_home_module`. What
  its optimized MIR has that the crate's hasn't, `Mir::new` reads as
  rust-js does: a switch that goes one way whatever it tests, which drop
  elaboration leaves, is that way, and the discriminant it read nothing
  else does goes; `#[rustc_inherit_overflow_checks]`
  arithmetic wraps, as the release profile's (ADR 0011); and its drops,
  elaborated by rustc already, each drop what's there, with no dataflow of
  rust-js's own, which its optimized switches wouldn't let be.
- **Its facts are read from its MIR** where the crate's are read from
  THIR: which parameters it steps (`stepped_params`), of each function
  value it names and of each struct it makes of a parameter, `Peekable {
  iter: self }`, stepped as the struct's `next` steps it; a std impl's drops
  as a library impl's (ADR 0100); and a crate given std's bodies' writers
  take whether they're pretty, as any program may ask `{:#?}` of a
  `Peekable`.
- **The vocabulary**, what a lowered std body bottoms out on, is what
  rust-js lowers of any body: `Iterator::next` of a type parameter, its
  dictionary's; `FnMut::call_mut`, the closure called; `Try::branch`, `?`
  (ADR 0364); a `Drop` terminator, the value's drop; a function that
  never returns, the panic (ADR 0012).
- **One shape per std type, never per call site.** A type whose bodies
  are compiled, `is_compiled_std_type`, is std's struct, `{ iter, peeked
  }` of a `Peekable<I>`, wherever a value of it is made or read, in
  generic code or concrete: no longer a `$iter` of a slice's items, nor
  `$peekable`'s object, nor an adapter a struct holds (ADR 0542). Its
  `peeked` is boxed as a generic `Option` is (ADR 0051). Its `Debug` and
  its clone are its derives' bodies, `peekableDebug` and `peekableClone`,
  dictionaries of the library's.
- **A compiled iterator's struct is a JS iterator, where it's made.** One
  whose `Iterator` impl is compiled, `is_compiled_std_iterator`, is
  `$stdIterator(struct, next)` where std's own body makes it, `peekable`'s
  and `clone`'s: itself, with `Iterator.prototype`, its `next` the
  compiled `peekableIterator_next` given the dictionaries the body has,
  so `for...of`, `.toArray()`, JS callers and the `.d.ts` see what they
  saw. So a value of it is stepped as any JS iterator is, wherever it's
  stepped, with no dictionary of what it holds: `Iterator`'s methods of
  it never resolve to the library (`resolves_to_crate`), and the
  recognizers give the idioms, `$next(p)`, `p.toArray()`, `p.reduce(..)`;
  what a use site needs of `I` is the compiled impl's alone, `next_back`'s
  `IDoubleEndedIterator`. A recognizer rewrites a call, never a value's
  shape: `peek()`, `peek_mut()`, `next_if()` and `next_if_eq()`, whose
  bodies reach memory or read as `next`'s, are `$peek(p)`, `$peekCell(p)`
  and `$nextIf(p, f)` over the struct, stepping it by its own `next`;
  `size_hint()` and `len()` are `$peekableHint`'s. Consumed by value,
  `p.sum()`'s, it's dropped after its method as the crate's own iterator
  is (`dropped_after`): by its fields' drops, given where it holds only
  type parameters' values, which a library's consumer sees too.
- **Sources stay recognized, stepped by dictionary.** `slice::Iter::next`
  and `Range::next` reach memory, or are primitives: a compiled
  `Peekable<I>::next` steps `I` through the dictionary its caller gives,
  `{ next: $next }` of a `$iter`, `{ next: $rangeNext }` of a range, never
  through an inlined slice body. Polymorphic optimized MIR leaves
  `I::next` unresolved, which is what the measure counts as the vocabulary.
- **A std trait's provided method compiled is a function over `Self`**
  (ADR 0185), `iterator_peekable(it, IIterator)`: called so on a type
  parameter, whose dictionary is passed on, and on a type, whose
  dictionary is made where it's called (`resolves_to_crate`, `trait_call`);
  its `Self` is given as it is where its signature holds it, a result
  `Peekable<Self>` (`given_as_it_is`).
- **A std-private trait on a type parameter is given as any bound is**
  (ADR 0049): the dictionary of the impl rustc resolves for the caller's
  type, in the caller's typing environment, so a `Pattern` of a `char` is
  `char`'s impl and of a `P: Pattern` the caller's own dictionary passed
  on; where a type meets only the blanket impl, `SpecFold`'s `impl<I:
  Iterator>`, that one, the specializations being std's optimizations of
  it, which `rustc_specialization_trait` says change nothing a program
  sees. The `spec` count says how many bodies need one.
- **`classify`'s refusal moves to the leaf**: a method it doesn't know is
  lowered from its body; what's refused is the memory operation in that
  body, or in one it calls, naming the std function it's in. What
  addresses memory in MIR has one owner, `pointers::mir_addresses_memory`,
  beside `raw_pointee`: the measure asks it, and the leaf's refusal does.
- **The built-in library shares nothing a program's values cross into**:
  it counts no `Rc`, copies no tuple and checks no metadata hash of
  `core`'s, which `Foreign::any` and `Foreign::check` leave it out of; only
  its own items are a library's, `Foreign::in_library`.

What's compiled first is `Peekable`: `Iterator::peekable`, its `next`,
`next_back` and `rfold`, its clone and its `Debug`; what the struct's own
`next` steps, `count`, `nth`, `last` and `fold`, are the JS iterator's
idioms, as any JS iterator's. `$peekable`, `$peekableClone` and
`$peekedItem` are gone, and so is the refusal of a `peekable` of the
crate's own iterator, and of a `peekable` generic code gives back of a
type with a destructor (`test/corpus/peekable_given_back_drops.rs`). The
app's JS reads as it read, but for a `peekable()`, which is
`iterator_peekable(it, { next: $next })`, and a `next_back()`, the
compiled impl's call.

## Why

- **It's exact by construction**, the argument ADR 0364 made for the
  app's bodies: the order, the drops, the panics and the `Debug` of a
  `Peekable` are std's, not rust-js's reading of std's docs.
  `test/corpus/std_peekable.rs` steps one of a slice's iterator and one
  of a range, in generic code and concrete, against native Rust, and the
  peekable cases that were, `held_peekable`, `peekable_drops`,
  `peekable_lazy` and `peekable_len`, run as they ran.
- **One mechanism replaces an enumeration**: what `$peekable` and
  `$peekableClone` modeled by hand, with every direction, clone and size
  hint rust-js hadn't reached, `core::iter::adapters` says in 7,700 lines
  of mostly safe Rust.
- **Coverage stops being linear in ADRs**: `Iterator` has 70 provided
  methods and 25 adapters; a crate's generic code calls any of them, and
  each is a line in `COMPILED_STD` once its body is safe.
- **It keeps the north star where it matters**: the app's JS keeps its
  idioms, since the recognizers stay; `@rust-js/std`'s JS is a library's,
  read as `@rust-js/runtime`'s is.

## Alternatives

- **A std port in safe Rust over JS primitives, compiled by rust-js as a
  library**, as Scala.js wrote the Java library in Scala. No new reading
  of MIR: a library under ADR 0100, its functions keyed by std's paths.
  Rejected: Scala.js couldn't compile the JDK; rust-js can read std. A
  port is 10 to 20 thousand lines kept against each std release, and every
  `Debug` format, panic message and edge case drifts from the real thing.
  Reading std's MIR cost one reading, once, and is exact.
- **Keep enumerating**: an ADR per adapter, direction, clone and holder.
  Rejected: 42 of the last 145, with no end the list shows.
- **Monomorphize std bodies into each app**: what rustc does. Rejected
  for the reasons of ADR 0049: larger output, no JS-callable generic API,
  and a std body in the app's JS where the app wrote one line.
- **Lower a std body into each app's output** where it's called, instead
  of a package. Rejected: each app would carry its own `Peekable`, and
  two apps' JS would disagree where a library's agrees; the runtime's
  helpers are a package for the same reason (ADR 0103).

## Consequences

- **The compiler builds in `std/manifest.json`**: a change to what std's
  bodies lower to is `bun run std` and a rebuild, which
  `test/std-package.test.ts` holds; the first build of a checkout needs
  the committed manifest, as it needs `runtime/index.js`.
- **`@rust-js/std` is installed beside `@rust-js/runtime`**, by
  `@rust-js/create`, the distribution and the examples; `rust-js build`
  checks its version as it checks the runtime's.
- **`none` bodies**, 27 in the registry, `to_lowercase` among them, and
  `fmt`'s builders outside it: a sysroot built once per toolchain from
  rust-src with `-Zalways-encode-mir`, as the playground's is staged
  (`wasm/build.sh`), or they stay recognized.
- **`opaque` bodies**, 5: `char`'s Unicode tables and `HashMap::entry`
  through hashbrown; each is `none` one call down, the same answer.
- **`{:?}` of a std adapter whose `Debug` isn't compiled**, `Copied`'s, as
  a `Peekable<Copied<I>>` shows it: refused, until `Copied`'s derive is
  in `COMPILED_STD`, which is the next adapter's one line.
- **The `*Dropping` helpers take an optional drop**, `$nthDropping`'s, as
  every drop parameter is (ADR 0300): a std body calls them with what its
  caller gave, which of an item without a destructor is nothing. The
  crate's own generic `nth` had crashed on it.
- **What it doesn't move**: 68 of rustc's 99 known failures are
  addressable memory, and the crate corpus stops on yoke, memchr and
  powerfmt, raw pointers all. A compiled std moves the adapter and trait
  tail, not memory, which needs a decision of its own.
- **Next**: each adapter after `Peekable`, a line of `COMPILED_STD` each,
  retiring its helper, `$fuse`'s ([0566](0566-fuse-is-stds.md)) and the
  held adapters' (ADRs 0542 to 0549), and `Iterator`'s provided methods
  the recognizers don't rewrite. Tidy of a std body's JS as of the app's,
  `next_back`'s dead `if (option != null)` after its early return, is
  [0564](0564-a-std-body-is-tidied-as-the-apps-are.md).

## What holds it

- `test/std-package.test.ts`: the committed `@rust-js/std` is what the
  compiler writes of std's bodies, a `Peekable` is made by its `peekable`,
  imported from it, and stepped as a JS iterator, `$next(p)`.
- `test/corpus/std_peekable.rs` and `peekable_given_back_drops.rs`,
  against native Rust; `held_peekable`, `peekable_drops`, `peekable_lazy`
  and `peekable_len` unchanged in what they print; `examples/lexer.rs`'s
  snapshot.
- `docs/std-coverage.txt`, each method's body kind beside whether it's
  known, an idiom's or a compiled body's, held equal by
  `test/std-coverage.test.ts`.
- The architecture test: recognition alone names std's functions and
  types, `compiled_std`, `is_compiled_std_type`, `std_home_module`,
  `memory_intrinsic` and `addresses_memory`; `pointers.rs` alone reads
  MIR for memory, `mir_addresses_memory`; and the dispatchers didn't grow.
- Mutations `dead-switch-kept`, `dead-discriminant-kept`,
  `std-struct-not-a-js-iterator`, `compiled-iterator-stepped-by-library`,
  `compiled-iterator-a-user-iterator`, `compiled-iterator-not-dropped-after`,
  `compiled-iterator-of-destructors-not-dropped-after`,
  `std-provided-self-given-by-trait` and `std-held-parameter-not-stepped`;
  `$peek`'s, `$nextIf`'s and `$peekableHint`'s own. What `$peekable` and
  the refusal of a generic `peekable` given a destructor held, their
  mutations held too, and go with them.

## Amendment: a source's own size hint

Case: C ([0262](0262-when-rust-and-js-disagree.md)): the answer was
`(0, None)` where Rust gives the source's hint; it's Rust's now.

`Countdown(3).fuse().size_hint()`, of an iterator of the crate's with a
`size_hint` of its own, was `(0, None)`, and so was a `Peekable`'s: the
use site gave the compiled body `{ next }` alone, and its `I::size_hint`
was `$sizeHint(iter)` of what it holds, which can't know. Issue #92.

- **The crate's iterator's `Iterator` dictionary carries its own
  `size_hint`**, where its impl has one (`own_size_hint`, ADR 0170's
  amendment): `{ next: countdownIterator_next, size_hint:
  countdownIterator_size_hint }`.
- **A std body's `I::size_hint(x)` of an `I` given as it is is `$sizeHint(x,
  IIterator)`**, which asks the dictionary's own before what it can find
  of `x`; a `Peekable`'s hint, `$peekableHint(peekable, SelfIterator)`.

`test/corpus/adapter_size_hints.rs` asks each, directly, in generic code
and after a `peek`, against native Rust, whose hints, each shown twice,
are held as `value$4` and the like: the readability measure's renamed
share grows, 825.2 to 825.5 in ten thousand lines; mutations
`own-hint-not-in-dictionary`, `size-hint-undictionaried`,
`peekable-hint-undictionaried`, `size-hint-dictionary-unasked` and
`peekable-hint-dictionary-dropped`.

## Amendment: a struct's hint is of what it holds

A compiled std iterator, `Fuse` or `Peekable`, was taken to know its
hint, its struct's own; but that's its source's, and a `map` or a
`filter` made here is a JS iterator, which knows nothing of its length:
`size_hint()` was `(0, None)` and `len()` panicked (#154). Whether it
knows is `held_hint_known`'s answer, of `size_hint()` and `len()` both:
of a `$iter`, a range, the crate's own iterator, a generic one's
dictionary, an explicit adapter, or another such struct; a struct of a
lazy std adapter is refused. Held by `test/corpus/held_hint_refused.rs`.

## Amendment: a JS iterator given as it is is itself (#276)

A compiled std struct's source can be a JS iterator, such as a chain, a
`$rev` or a `Vec`'s `$iter`, whose dictionary is JS's own stepping,
`{ next: $next }`. `$given` wrapped that source in a new `$iterator`
that stepped `next`. The source's own `last`, `nth`, `nthBack` and chain
`sides`, std's for its kind, were hidden behind the wrapper. So
`chain(..).peekable().last()` dropped each item as it passed it, where
std's `Chain::last` takes each side's `last` and drops the first side's
after. And `rev().peekable().nth(3)` dropped a `Vec`'s items back to
front, where its `nth_back` drops what it passes as one slice (fuzz
seeds 51951, 50899, 51010 and 46168). Following ADR 0262's "results
change" case, Rust's order is kept: `$given` of a value whose
dictionaries are only JS's stepping, with no length, gives the value
itself. A dictionary that names any method of the crate's still gets
the wrapper that calls it.

The `peekable_source_own_steps` corpus case checks a chain's `last`
and a `Rev`'s `nth`, through a `Peekable` and through a `Fuse` then a
`Peekable`, against native Rust. The mutation `given-js-iterator-wrapped`
is caught.

Readability: the case's `Vec`s of values with a destructor add `$iter`
(77.6 to 77.8 in 10,000 lines) and renamed temporaries (715.6 to 716.5),
as each such case does. The output reads no worse.

## Amendment: a compiled `Map`'s and `Inspect`'s fold is std's (#264)

std's `Map::fold` is its source's `fold`, each item given its `f`, and
`Inspect::fold` likewise. The compiled structs had no `fold` step,
though: their compiled `fold` takes the call's own `B` and `F`, which the
struct's steps can't name. So a `sum` through `map(..).peekable()` stepped
`next`, and the crate's own `fold` never ran (fuzz seeds 47014, 51072 and
53603). `Peekable` and `Fuse` already had std's `fold` by a runtime
helper chosen in `std_iterator_steps`' table. `Map` and `Inspect` now
join that table, `$mapFold` and `$inspectFold`, recognized by
`mapping_adapter`. This follows ADR 0262's "results change" case. A
dictionary that gives each compiled `fold` its call's own type parameters
would serve every adapter. It remains open, and these helpers would go
with it.

`own_fold_through_adapters` folds a map and an inspect through a
peekable, the crate's own `fold` reached and its value dropped once,
against native Rust. The mutations `map-fold-unmapped`,
`map-fold-not-std` and `inspect-fold-not-std` are caught.

## Amendment: a compiled `FlatMap`'s fold is std's too (#297)

std's `FlatMap::fold` is its `FlattenCompat`'s: the front iterator's fold,
then its source's, each item made an iterator and folded, then the
back's. The compiled `FlatMap` had no `fold` step, so a held `fuse()` over
`Count.flat_map(..)` stepped `next`, and the crate's own `fold` never ran
(fuzz seed 65222). `$flatMapFold` joins the table, given the
`IntoIterator` dictionary of what each item makes. When the crate's own
`fold` takes a map's source and drops it, the map is now marked closed,
so closing it after doesn't drop that source a second time, which the
new case showed. `own_fold_through_adapters` adds the shape, and
`flat-map-fold-not-std`, `flat-map-fold-items-unmade` and
`map-fold-source-dropped-again` are caught.
Readability: the line's `Vec` adds `$iter` (87.6 to 87.7 in 10,000 lines),
as each such case does.

## Amendment: std's bodies are lowered of a `core` built to unwind (2026-10-30, #424)

The `core` the toolchain ships for wasm32 is built to abort. Each call in
its MIR unwinds nowhere (`UnwindAction::Unreachable`), so it has no
cleanup to lower. A panic that unwound through a compiled body, a `Fuse`'s
or a `Peekable`'s over an `Inspect`, dropped nothing the body held: native
drops 32, 33, JS 33 alone. Results change, so Rust's behavior is kept (ADR
0262). The user's decision was a std built to unwind.
- **`--std-bodies` lowers std's bodies of a `core` built to unwind**
  (`std_sysroot.rs`), given as the session's `--sysroot` where none is.
  `@rust-js/std` is `core`'s bodies alone, `std/lib.rs` is `#![no_std]`,
  so what's built is `core`, of the toolchain's `rust-src`, `-Cpanic=unwind`,
  its metadata alone with each body's MIR and MIR inlined, and a
  `compiler_builtins` that's a crate and nothing more, as the panic runtime
  is (ADR 0090). In-process, no Cargo or network. It's built once, 2.4 s, and
  kept beside the compiler, keyed by the toolchain, the target and how it's
  built.
- **Std's own items are keyed by their crate's name and def path**,
  `core::iter::adapters::inspect::{impl#3}::next` (`lower::library::key`),
  where they were keyed by `DefPathHash`. A program's session, on the
  shipped std, then finds what `@rust-js/std` compiled, though the two
  `core`s' crate ids differ. The same source gives the same paths. A
  program's session stays on the shipped sysroot, so no host changes.
- **A cleanup that tests an enum's discriminant** before it drops a
  variant's field, `switchInt` of `discriminant(_)`, is an `if` of the
  variant's test (`switch_test`). The discriminant is read as a value,
  never written to a temporary the function doesn't declare. A drop
  flag's is tested as it's set, `if (flag) { drop(x); }`, as before, which
  `settled_flags` settles.
- **A closure only a local of a std body holds,** one inlined there, is
  collected with the body, `sum`'s in `Filter::count`.

208 `catch`es in std/iter.js, and its MIR's inlining differs where a call's
cleanup makes it costlier: `Peekable::next_back`'s `or_else` stays a call,
`next_back(..) ?? fallback()`, as `test/std-package.test.ts` now holds.
Still open in #424: an accumulator or a source a compiled body moves into
a dictionary's `fold`, which drops neither as a panic unwinds. Held by
`panic_in_std_bodies_drops.rs` against native Rust and the mutations
`std-core-aborts` (`std_sysroot.ts`), `std-core-shipped` (`main.ts`),
`std-keyed-by-hash` (`lower/library.ts`), `cleanup-switch-bool-only` and
`cleanup-discriminant-written` and `cleanup-flag-tested-negated`
(`lower/mir/drops.ts`), and
`inlined-closures-unfound` (`lower/analysis.ts`).

## Amendment: what a compiled body moves into a fold, dropped as it unwinds (2026-10-30, #424)

With cleanups in std's compiled bodies, what they move into a call was
still lost on a panic. `inspect(..).fuse().fold(..)` dropped the
inspected item alone, where native drops it, then the accumulator, then
what's left. `fuse().count()` lost what's left. Results change, so Rust's
behavior is kept (ADR 0262):
- **An iterator a std method takes by value, which rust-js drops after it**
  (`dropped_consumed`), is dropped though the call throws, in a `finally`,
  as the method that took it drops it as it unwinds. That applies to one a type
  parameter's or one whose steps run what may panic (`steps_may_panic`):
  `try { return Array.from(iter); } finally { iter.return(); }`.
- **A compiled `fold`'s accumulator is dropped by the runtime fold's
  `unwind`:** its dictionary entry, `(iterator, init, f, unwind) => ..`,
  gives `unwind` as its `B`'s drop (`evidence_args_given`'s drops given by
  index), and `$stdIterator`'s `fold` passes it on. A fold of a type
  parameter's iterator, `Fuse::fold`'s of what it fuses, is
  `$foldDropping`, whose steps are anyone's. An `rfold`'s is the caller's,
  as before.

15 corpus cases read so, 83 lines more, each a consumed iterator's
`finally`. Held by `panic_in_std_bodies_drops.rs` against native Rust and
the mutations `consumed-unclosed-on-throw` and
`consumed-unwinding-everywhere` (`lower/mir/iter.ts`),
`compiled-fold-given-no-unwind` (`lower/iterators/knowing.ts`),
`given-drop-ignored` (`lower/traits/calls.ts`),
`std-iterator-fold-unwind-lost` (`runtime/std_iterator.ts`) and
`fold-of-a-param-unwinding-unseen` (`lower/iterators/mir_steps/collect.ts`).

## Amendment: a compiled iterator's hint is `held_hint_known`'s alone (2026-11-06, #799)

`size_hint()` from MIR asked `hints_its_size`, of a `$iter`, a type
parameter's, an explicit adapter's or a compiled struct's own hint,
beside `adapted_hint_known`, whose `held_hint_known` asks each of those
already. It's gone: what's asked is `array_source(..)` or
`adapted_hint_known(..)`, one answer. Its mutations,
`size-hint-of-projection-refused`, `compiled-iterator-size-hint-refused`
and `held-size-hint-refused`, mutate `held_hint_known` now, in
`iterators/knowing.rs`. Nothing that runs changes (case N).

## Amendment: a compiled iterator's own `count` is passed on (2026-11-09, #1048)

`let mut it = v.into_iter().flat_map(Own); it.next_back(); it.skip(1).count()`
counted by `next`, where native Rust's `Skip::count` asks its source's
`count`, the compiled `FlattenCompat::count`, which asks each inner
iterator's own, the crate's `Own::count`, printing what it prints (native
`own count 1`, `own count 3`, `own count 3`, JS none). `$stdIterator`
forwarded the compiled `count` but didn't say it's its own, as it does of
its `nth` and `last`, so `$skipping` (which passes a source's `count` on
only where it's `ownCount`) stepped it. **A compiled iterator with a
compiled `count` owns it, `$owns(it, "count")`**, as a `$flattenEnds` does
already. ADR 0262's case: none, the same count by the body std runs. The
corpus's JS is the same. Held by `flatten_count_takes_each_inner_count.rs`,
of a held `flat_map`'s, `flatten()`'s and `fuse()`'s, and the mutation
`std-struct-count-not-passed-on`.

## Amendment: std's crate hash is its inputs' alone (2026-11-09, #1094)

`std/manifest.json`'s `crate_hash` changed with every checkout that ran
`bun run std`, a commit's alone, `a6e126…` from one worktree and `ebc393…`
from another, though nothing it's made of had. It was rustc's hash of the
crate, which is of where the crate was compiled, of its options, of
`RUSTC_BOOTSTRAP`, which `.cargo/config.toml` gives what Cargo runs and a
test run has, and of each crate it's built on: the `core` built to unwind,
compiled where the compiler is, from where the toolchain is on the machine.
Remapping those paths away made the hash steady but lost what rust-js reads
of `core`'s source, its locals' names, so std's JS read `tmp$1` where it
read `count`.

Nothing checks std's hash: it's a builtin's, which `Foreign::check` skips,
as no consumer loads std's metadata. **So `@rust-js/std`'s manifest's crate
hash is of what it's made of alone (`Manifest::of_its_inputs`): its inputs,
by their relative paths and their contents' hashes, and the toolchain's
release.** And std is built one way, `scripts/std.ts`, which `bun run std`
and `test/std-package.test.ts` both run, in the checkout's root.

The test compares the hash too now, and "std's crate hash is the same built
anywhere, with or without RUSTC_BOOTSTRAP" builds std in two places, one
deeper and with `RUSTC_BOOTSTRAP` given, each the committed one; it fails
with rustc's hash, mutation `std-crate-hash-rustcs`.

## Amendment: std's bodies are `@rust-js/std`'s entry's, with the helpers (2026-11-09, #1095)

A module imported std's bodies by their file, `from
"@rust-js/std/iter.js"`, and the helpers from `@rust-js/runtime`. **Now
both are one import of one package's one entry, `from "@rust-js/std"`**
(ADR 0103's amendment): the package's `index.js` re-exports its helpers'
module, `runtime.js`, and `iter.js`, which imports the helpers beside it.
std's manifest names `std.js`, its link to `runtime.js` a file of the
package's own, and is built into the compiler as before.

## Amendment: a slice's chunks held know their length (#1100)

`len()` of a compiled iterator asks its struct's hint, which it knows where
`held_hint_known` finds the size of each iterator it holds. A slice's
chunks and windows, `$iter` of the array of them made where they're made
(ADR 0335), weren't among those, though their JS knows its length as an
array's items' does. So rand_core 0.9's `fill_via_chunks`, `let zipped =
dest.by_ref().zip(src.by_ref()); let num_chunks = zipped.len();` of a
`Zip<&mut ChunksExactMut<u8>, &mut slice::Iter<T>>`, was refused, "of a
lazy adapter", and with it rand_core. What `iterates_items` says is made an
array of items, `$iter` of it, is now one whose hint is known. Held by
`test/corpus/zip_len_of_chunks.rs`, the function and zips of `chunks`,
`windows`, `chunks_exact` and a stepped `rchunks`, against native Rust in
both builds. Mutation `chunks-hint-unknown` (`lower/iterators/knowing.ts`).

## Amendment: a module reserves no name for std's exports it doesn't import (2026-11-11, #1156)

Every module reserved a name for each of `@rust-js/std`'s exports before
lowering, as it does for a library's, whose imports are named then (ADR
0100). std exports what its compiled bodies are, `lines` and, with #1156,
`io::take`'s `take` and `io::chain`'s `chain`, which no crate names, so
a function a crate's scope struct holds (ADR 1043), `W::take`, became
`take$1` where nothing imported std's. Only a library's exports are
reserved now (`Foreign::builtin_modules`); one of std's a module does
import, which its lowering reads as a symbol, is named apart by the
linker where a name of the module's has it, `Chain as Chain$1`. The
corpus's JS is the same; held by the mutation
`std-exports-reserved-everywhere` of `refused_drops_and_displays.rs` and
`consumer_drops_what_is_left.rs`, which #1156's `take` changed without it.

## Amendment (2026-11-13, #1458): a std maker of a compiled struct is given its type parameters' drops

Case ([0262](0262-when-rust-and-js-disagree.md)): preserve. A compiled
std struct that closing drops (ADR 0871) drops its fields when it's
closed, by drops its maker gives it. A trait's provided maker,
`Iterator::take`, is given `Self`'s drop by the rule for a trait's default
body. A free function that makes one, `repeat_n`'s `RepeatN<T>`, was
given none, as its body drops nothing but where it makes no items:
compiled, `repeat_n(L(1), 0)` would never drop `L(1)`, nor a `RepeatN`
closed before its end the element it holds. A std function that gives back a compiled std iterator
is now given a drop for each type parameter it holds that isn't `Copy`
(`analysis/drops.rs`; whether the struct holds it is `generic_facts.rs`'s
`holds_type_parameter`), so the struct's close drops it. The crate's own
functions are left as they are: their callers give what their bodies'
calls ask.

No compiled std free function makes an iterator before `RepeatN` is
compiled, so this changes no output: the committed std, every corpus
snapshot and the plain build are byte-identical with it. Its case and
mutation are `RepeatN`'s, in the commit that compiles it: with
`std-maker-given-no-drop` applied and std rebuilt, the case fails in the
build of std's bodies (`repeat_n(L(1), 0)` never drops `L(1)`) and the
twin net's consumers table fails 14 of its 80 batches, std's bodies
leaving `L(7)` undropped where the runtime's helpers drop it.

## Amendment (2026-11-13, #1420): `core::net`'s addresses

`Ipv4Addr`, `Ipv6Addr`, `IpAddr`, `SocketAddrV4`, `SocketAddrV6` and
`SocketAddr` are plain data in `core::net`. Calling any of their functions
was refused. quickcheck 0.9's `Arbitrary` for them calls `Ipv4Addr::new`,
so quickcheck was refused whole, and so were itertools' 285 quickcheck tests
(#1420). They're now compiled into `@rust-js/std`'s `net` module, as `io`'s
`Cursor` is (#1156): `COMPILED_STD_NET`, the one catalog of them. That
covers their constructors, accessors and tests (`octets`, `segments`,
`to_bits`, `is_loopback`, `is_private` and the rest), `From` (of an
address, an array or a number), `==`, `Hash` and `Clone`. Each address is
its struct, `{ octets: [127, 0, 0, 1] }`, wherever a value of it is made
or read; an `IpAddr` is std's enum, `{ TAG: "V4", _0 }`. This was the
lead's option A on #1420, over a hand-written model (#1458's direction).

Still refused, #1644: `{}`, `{:?}`, `parse()` and `<` of them. Their bodies
reach core's internals, `compare_bytes`, `SpecWriteFmt` and the address
parser, which rust-js doesn't lower yet. So do sockets, `TcpStream` and the
rest, which JS can't give as Rust's are.

No observable difference (ADR 0262): std's own bodies run. Corpus case
`net_addresses_compiled` runs against native Rust in both builds.

## Amendment (2026-11-13, #1648): `next_if`'s item dropped as its test panics

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior is preserved.

Std's `next_if(f)` takes the next item into a local, then asks `f` of
it, so where `f` panics the item drops as the panic unwinds, before what's
left of the iterator. `$nextIf` took the item and called `f`, so a panic
there leaked it: `drop 5`, `drop 6` where native Rust prints `drop 4`
first. `$nextIf` and `$nextIfSome` are given the item's drop, where the
test may unwind (`call_may_unwind`), and run `f` by `$unwinds`, which
drops the item as it unwinds. A test that can't panic is given none, so
its JS is as before.

`test/corpus/next_if_drops_its_item_as_its_test_panics.rs` holds it
against native Rust in both builds, of items that may look like `None`
too. Mutations `next-if-item-leaked-as-test-panics` and
`next-if-some-item-leaked-as-test-panics` (`runtime/next_if.ts`),
`next-if-given-no-drop` and `next-if-drop-given-a-test-that-cant-panic`
(`lower/mir/std_calls/iterators.ts`).

## Amendment: the catalog's rows are resolved once, and a row that resolves to nothing is an internal error (2026-11-13, #1681)

Review #1671's change 2 makes std's compiled bodies the default meaning
of an iterator chain, so the catalog of which bodies are compiled has to
be exact. It was resolved by name, and silently: a row whose type, trait or
method a toolchain didn't have compiled nothing, with no error, and each
provided method was found again by name wherever it was asked.

**`compiled_std/rows.rs` resolves every row once a session**
(`compiled_rows`): `COMPILED_STD`'s, `COMPILED_STD_IO`'s and
`COMPILED_STD_NET`'s to their methods, each provided method a catalog
names (`HELD_COMPILED`'s makers too), `io`'s provided methods, and each
free function. A row that resolves to nothing panics as an internal error
naming the row, every compile of std's bodies. `compiled_provided_method`
reads the table and gives a `DefId`, and a pair no row names is an internal
error too. Resolving everything, whatever the build, holds `MEANING_ONLY`'s
rows as well.

It found two dead rows on Rust 1.100, which compiled nothing:
`("Skip", ExactSizeIterator, "len")` and `("Take", Iterator,
"__iterator_get_unchecked")`. Neither impl defines that method. Both are
removed, `@rust-js/std` is byte for byte the same, and so is every
compile. Mutations `net-addresses-uncompiled` (moved here) and
`catalog-private-trait-unfound` are caught by `test/std-package.test.ts`.

## Amendment: a clone and a move of std's struct know what it knows (2026-11-16, #1976)

A use site gives std's compiled struct its back, its length and its
compiled `rfold`, `nth_back` and `try_rfold`, through `$knowing`. The
struct's `Iterator` dictionary can't hold them, because each comes from
the use site's dictionaries. But std's compiled `Clone` made its struct
only "as the one it clones is", `$stdIterator({ iter, f }, map.$steps)`,
and a move apart, `$copy`, made it from the steps alone. Both lost what
the use had given. A `rev()` over a clone of a `map` its use gave a back
then called `.back` on nothing: `TypeError: it.back is not a function`,
where Rust steps it. This happened in two ways: through generic identity
functions (the metamorphic net's opaque rewrite), and with generic code
given the chain beside the clone, `show<I: Iterator + Clone>`, which
makes the chain std's struct.

`$stdIterator`'s `knowing` now keeps what it's given as `$knows`, each
entry still unbound, what an earlier use gave kept where a later one gives
none. `$stdIterator(struct, steps, knows)` replays it, bound to the new
struct, so a clone steps only itself. The `Clone` the aggregate lowering
compiles passes `self.$knows` with `self.$steps`, and `$copy` passes
`it.$knows`. What a clone knows comes from where `$knowing` put it, never
from its type, because generic code can't know it statically. Outside
`@rust-js/std`'s runtime and its compiled `Clone`s, every compile is byte
for byte the same.

No Rust program reaches `$copy`'s route today: a move of std's struct is
of its fields where the crate's code makes it. So a unit row of
`test/runtime-package.test.ts` holds that route, and the other two. Corpus
case `clone_of_a_compiled_adapter_knowing_its_back` holds both routes
against native, each row a function of its own. Each steps the clone from
its back, by a `rev()`'s `next`, `nth` and `fold`, and checks the original
is as it was, and the reverse. `len()` there is refused, `ExactSizeIterator`
of a `Copied<Map<..>>`, a refusal of its own.
Mutations `std-iterator-clone-unknowing`, `std-iterator-move-unknowing`,
`std-iterator-knows-the-last-use-only` and `std-clone-unknowing` are each
caught.

## Amendment (2026-11-22, #2305): the catalogue is kept with std

**Every compile used to walk std's MIR to find which of std's bodies
`@rust-js/std` compiles:** the catalog's rows, and each private function a
compiled body reaches (`compiled_std.rs`'s `walk`). That took about 6.3 ms
of each compile's lowering, the same answer every time for a given std.
(#2305's profile said 26 ms, 20% of compiler CPU, but it measured a
release build under samply. The debug compiler the tests run measured
8 ms before #2306, and 6.3 ms after.)

**Now `bun run std` keeps the walk's answer in `std/compiled-std.json`,
and every compile reads it.**

- **What's stored.** The base group: what every adapter's roots reach but
  `MEANING_ONLY`'s. Then each `MEANING_ONLY` adapter's group: what its
  roots add to the base. Each id is stored by its `DefPathHash`, under the
  name and hash of each crate the ids are of (core and alloc).
- **Which compile reads it.** A compile whose loaded crates have those
  names and hashes reads the base, plus the group of each adapter it calls
  (`called_for_build`). Any other compile walks, as before: std's own build
  among them, since its `core` is built to unwind. The walk stays the one
  owner, and a mismatched key is a cache miss, never a second rule.
- **Why groups combine.** The walk admits each body on its own conditions,
  never by what else it found. So what a union of groups' roots reaches is
  the union of their walks.
- **Order.** The kept set's order isn't the walk's. Only std's own build
  reads the order, as the order to compile in, and it walks. Every other
  reader asks membership (`is_compiled_std`, `has_compiled_bodies`).
- **How it's made.** `scripts/std.ts`'s `catalogueBuild` compiles an empty
  crate with `--std-catalogue <file>`, which writes each group's walk. The
  flag is dev-only, std's build's, as `--std-coverage` is: no host passes
  it, as every compile reads the catalogue embedded in the compiler
  (`include_str!`), as it reads std's manifest.
- **Proof.**
  - test/compiled-std.test.ts makes the catalogue again and compares it,
    byte for byte. `bun run push`'s std check does too.
  - A probe over the whole corpus compared the kept set with the walk in
    every one of its 5,677 compiles: 0 differences, 14 of them taking
    groups (Copied, Cloned, Successors).
  - The compiler's build inputs (build.rs, CI's build cache key) include
    the file, as they include std's manifest.
- **Measured.** On the quiet 12-core M6, warm, against the same base: a
  corpus run took 37 s → 33 s wall, and its compiles 367 s → 324 s summed.
  Lowering went 264 s → 220 s.
- **Also.** The walk's own bookkeeping is now an `IndexSet`, and each
  body's reachable blocks are found once.
- **Mutations:** `catalogue-crates-unchecked`, `catalogue-groups-unread`,
  `catalogue-groups-unkept`.


## Amendment: a compiled consumer, `partition`, where its helper can't fill a side (2026-11-23, #2387, #2378)

`partition` into a side its helper, `$partition`, doesn't fill as std
does, the crate's own `Extend + Default`, a type parameter's, a set of
what has a destructor, is std's own body, compiled into `@rust-js/std`
(`COMPILED_STD_PROVIDED`'s `Iterator::partition`), with `Extend::extend_one`,
std's default `extend(Some(item))`, which it calls through the side's
dictionary, compiled beside it. So std's order is the program's by
construction: the left side's `Default`, then the right's, each item's
`extend_one` as it's kept, and as a panic in the predicate or a side's
`extend` unwinds, the item, what's left of what it steps, then the right
side and the left.

- **One rule, two forms.** `recognition::side_filled_by_helper` says which
  form each side is made in, and both places that decide ask it: where
  the call goes (`compiled_instance`, which otherwise answers every
  catalogued provided method's call std's body, whatever its `Self`), and
  where its sides are made (`fillable_sides`). A `Vec`'s sides stay the
  helper's arrays, its corpus's snapshots unmoved; the helper is the
  readable special case of std's body where the two can't be told apart.
- **A compiled consumer's `Self`.** A compiled adapter, `take`'s, is given
  its `Self` with a dictionary of its `Iterator`, `SelfIterator`, which it
  steps it by; a compiled consumer, `partition`'s, declares none and steps
  its `Self` as a JS iterator, as generic code holds one (ADR 0061). So a
  call of one is given the crate's iterator made one, the recognizer's
  `as_js_iterator`, `$iterator(up(), upIterator_next)`
  (`traits::evidence::given_self_iterator`); given it as it was, it threw.
- **The carrier agrees.** `recognition::crate_impl_called_by`, a list now,
  answers `partition`'s side's `Extend::extend::<Option<Item>>`, which the
  compiled body reaches, so the callers' carrier and the debug build's
  check (ADR 1115, #2376) see what the crate's `extend` is given.
- **One move, two authorities.** The corpus's
  `fold_drops_moved_function_first`'s `partition` row, into two `Vec`s,
  still `$partition`, is fed a chain of std's compiled `iterator_take` and
  `iterator_map` where it was the recognizers' `take` and `$holding`'s
  `map`, its output native Rust's still: `iterators::given_as_it_is`
  decides how what's given to a function is held by whether the function
  is in the catalogue, `is_compiled_std`, where the call goes by
  `compiled_instance`, which the side's rule gates. Two answers of where
  a call goes, a duplicated authority, #2398, which threads the call's
  arguments so every analysis asks `compiled_instance`.
- **Not `unzip`.** Its std body extends a tuple through core's private
  specialization trait `default_extend::SpecExtend`, which a generic
  compile can't resolve: its sides as #2377 made them, a crate's
  `Extend` refused, until #2395.

Held by the corpus's `partition_fills_each_side_as_std_does` (the crate's
`Bag`, through generic code too, a `HashSet`, a `BTreeSet`, a `Vec`, and a
`BTreeSet` of what prints its drop, its duplicate dropped as std drops it)
and `partition_into_own_extend_unwinds` (the crate's own iterator into
the crate's `Extend`, and a panic in the predicate and in `extend`, each
drop against native Rust), `lowering-collections.test.ts`' refusals of
`unzip`'s, and mutations `partition-of-vec-sides-compiled`,
`compiled-consumer-given-raw-self`, `partition-extend-unanswered`,
`filled-sides-of-destructors-by-helper` and `unzip-into-a-map-refused`.
Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's.


## Amendment: `unzip()`'s own body, a tuple's `Extend`, and a specialization taken for its default (2026-11-24, #2395, #2413)

`unzip()` into a side its helper doesn't fill as std does, a crate's
`Extend`, a type parameter's, a set of what has a destructor, is std's own
compiled body, as `partition()`'s is (`compiled_instance`'s gate, both
sides `side_filled_by_helper` or not). Its type-parameter side had been
the helper's array, which never called the impl a caller gave (#2395).

std's `unzip` is `(FromA, FromB)::extend(self)`, `default_extend`, so two
more of std are compiled:
- **std's impls of a trait for tuples**, `COMPILED_STD_TUPLES` (trait,
  arity), every method of each: `("Extend", 2)` now. A crate's own `extend`
  of a pair of collections, refused before, compiles (#2413). A tuple's
  word is `Tuple{n}`, and a trait argument that's a tuple of the impl's
  type parameters alone adds none: `tuple2Extend`, `tuple2Extend_extend`.
- **core's `default_extend::SpecExtend`**, a private trait std specializes
  on `TrustedLen`, which rustc leaves unresolved in generic code. It's
  `OBSERVABLY_DEFAULT`'s row (recognition/specialization.rs), so
  `optimized_default` takes its general impl, a blanket one: both impls ask
  `size_hint()` once, `extend_reserve` its lower bound, then give each item;
  the `TrustedLen` one by `extend_one_unchecked`, whose default is
  `extend_one` and which no stable crate can write. Each row names its
  source in rust-src, the item and its SHA-256; test/std-specializations.test.ts
  fails when std's source changes, and a row whose impls no longer add its
  bound alone is rust-js's internal error naming its source and reason.
  Corpus `extend_reserve_of_sides_generic_code_fills` runs both paths
  natively, against JS's default one, capacities included.
- `Extend`'s `extend_reserve` and `extend_one_unchecked` defaults join
  `COMPILED_STD_PROVIDED`, which std's impls write and ADR 1191 dispatches.

Corpus `unzip_fills_each_side_as_std_extends_it` (a crate's `Extend`, a
type parameter's `HashSet` and `String`, a set of what has a destructor,
and a pair's `extend`, directly and through `C: Extend<(u32, char)>`).
What steps a `partition()`'s or an `unzip()`'s source in place asks the
one gate too, `filled_by_its_helper` (`compiled_instance`'s), not whether
the method is compiled at all: a chain given to one its helper fills,
or held in a variable it steps, is made in place, as the helper steps a JS
iterator (corpus `unzip_of_a_chain_held_in_a_variable`; #2398 for these two).

Mutations: `helper-filled-consumer-holds-its-source`, `helper-filled-consumer-holds-its-variable`, `unzip-helper-of-mixed-sides`, `observably-default-row-unread`,
`unzip-crate-extend-unnamed`, `tuple-word-its-text`; `partition-param-side-refused`
is gone with the refusal it mutated.

## Amendment: whether a call gives std's body its value as it is is asked of the call (2026-11-24, #2398, commit 1 of 3)

`iterators::given_as_it_is` said whether a trait's own method compiled into
`@rust-js/std` takes `Self` as its signature says by the catalog's
membership alone, `is_compiled_std(id)`, while the lowering asks the call,
`compiled_instance(id, args)`: so a chain given to a `partition` into `Vec`
sides, which its helper fills, was analysed as given to std's compiled
body. Now `given_as_it_is(tcx, (id, args), param)` takes the call's
arguments where it's asked of a call (the crossing's `given_iterators`,
`given_iterator`, `given_as_iterator`; `body_queries/in_place.rs`) and asks
`compiled_instance` of them; asked of a definition, a body's own evidence
or an impl's dictionary, `None`, the catalog's answer, as before. The one
answer of a call's.

No output moves: the snapshot #2398 named moved back with f4e8f472b's
`filled_by_its_helper` gate, and a helper-filled `partition` or `unzip`
never reaches the crossing that asks it. So no mutation of the per-call
answer is caught by output; the rule is held by #2398's architecture test
(`is_compiled_std` read only in recognition), its third commit.
`naming-parameter-not-as-it-is` and `std-provided-self-given-by-trait`
retargeted.

## Amendment: a call's question and a definition's, each its own (2026-11-24, #2398, commit 2 of 3)

The catalog's membership, `is_compiled_std`, was read in 21 places outside
recognition, each its own way of asking where a std call goes or what a
std body is. Now each asks one of two questions, which aren't two ways to
one:

- **Of a call** (`compiled_instance(id, args)`, recognition/compiled_std.rs):
  where it goes, its instance's, its arguments' too: a trait call's
  `$provided` branch, a resolved instance of `Iterator`'s compiled consumer
  given `Self` as a JS iterator (#2387), the type facts of each call, the
  crossing, `stepped_as_a_js_iterator`, and a resolved fmt instance's
  (`sink_writes.rs`, `fmt_into.rs`: a resolved instance asks the instance).
- **Of a definition** (`compiled_std_body(id)`): what std compiles a body
  of, whatever calls it: display's writers, whose crate's JS, which impl
  methods are compiled, the coverage report, a body's own facts and
  evidence, the body being lowered.

What steps a source in place (`body_queries/in_place.rs`) asks a third,
`compiled_ahead_of_place(id, args)`: `compiled_instance`'s parts known
before what's made in place, `compiled_std_body` and not a helper-filled
`partition` or `unzip`, never a maker's `compiled_for`, which that analysis
decides; asked there, `compiled_instance` fed the analysis its own output,
and a cloned compiled `Map` became `$mapEnds`. `compiled_instance` is made
of it, so the two never part, and only that analysis may ask it (#2398's
architecture test, commit 3). No output moves.
