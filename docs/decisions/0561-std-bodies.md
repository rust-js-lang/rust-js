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
- **What it doesn't move**: 54 of rustc's 101 known failures are
  addressable memory, and the crate corpus stops on yoke, memchr and
  powerfmt, raw pointers all. A compiled std moves the adapter and trait
  tail, not memory, which needs a decision of its own.
- **Next**: each adapter after `Peekable`, a line of `COMPILED_STD` each,
  retiring its helper, `$fuse`'s and the held adapters' (ADRs 0542 to
  0549), and `Iterator`'s provided methods the recognizers don't rewrite.
  And tidy of a std body's JS as of the app's: `next_back`'s dead `if
  (option != null)` after its early return, folded.

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
