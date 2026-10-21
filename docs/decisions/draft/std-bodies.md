# std's bodies are std's own; a recognizer is a proved rewrite to an idiom

Status: Draft. Would extend [0364](../0364-mir-lowering.md),
[0314](../0314-std-data-structures.md), [0100](../0100-separate-crates.md)
and [0049](../0049-traits-and-generics.md); amend
[0023](../0023-strings-references-shared-state.md), whose "one known item
at a time, each with its JS meaning" would then be the primitives' rule
alone, and [0006](../0006-errors-and-unsupported-features.md), whose
"calling `..` yet" refusal would be reached only at a leaf of addressable
memory.

Case: C ([0262](../0262-when-rust-and-js-disagree.md)): a std function's
answer, its order of effects, its drops and its panics are Rust's, as its
body says. N for where the JS of a std body goes.

## Context

A std call is lowered one way today: `recognition::classify` says which
std function it is, by its lang item, diagnostic item or path, and its
type's module writes its JS (ADR 0314). What `classify` doesn't know is
refused: `rust-js does not support calling .. yet`. Each addition is an
ADR: of the 145 decisions 0400 to 0544, 42 are iterator adapters, each
hand-modeled with its own runtime helper, `Debug`, clone, size hint and
direction, `$fuse` (ADR 0473), `$peekable` (ADR 0543), a held
`Enumerate` (ADR 0542). std's `Peekable` is 300 lines of safe Rust,
`{ iter, peeked: Option<Option<I::Item>> }`, with those derived.

The bodies are there to read. The toolchain rust-js pins ships, in its
wasm32 metadata, the optimized MIR of every std function that's generic
or inlinable, and `rustc_private` reads it: `tcx.is_mir_available`,
`tcx.optimized_mir`. rust-js reads the app's own MIR already (ADR 0364);
a library's it never has, as ADR 0100 says (THIR isn't in metadata, and
MIR only for generic and inline functions), which is why a library is
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
`none`, no MIR of its own. Of 827 methods at 1.99.0: 231 safe, 21 spec,
5 opaque, 543 memory, 27 none.

The split is by stratum, not by chance:

| Stratum | Bodies | Examples |
|---|---|---|
| Values over the trait vocabulary | safe | `Option` 39 of 46, `Result` 34 of 36, `Pin` 16 of 16, `BTreeMap` 11, a slice's `contains`, `Vec::len`; and, outside the registry, `Peekable::next` and `next_back`, `Fuse::next`, `Iterator::try_fold`, `fold`, `count`, `position`, `max_by`, `Ord::max`, `Ordering::then_with` |
| Over a trait only std names | spec | `str::contains`, `starts_with`, `find` and `strip_prefix` through `Pattern`, unstable; a slice's `contains` through `SliceContains`, its `fill`, `concat` and `join`; `LinkedList::contains` |
| The containers and text | memory | `Vec::push` through `push_mut`'s raw pointer, `dedup_by`'s `copy_nonoverlapping`, a slice's `sort_by` (driftsort), `iter` (`Iter::new`), `str::split`, `trim` and `lines` (a `Pattern`'s searcher over bytes), `String::push_str`, every `Rc` and `Arc` |
| Non-generic, not inlined | none | `str::to_lowercase`, `Instant`'s and `SystemTime`'s, `String`'s 9 |

The registry's types are the primitives, so most of their methods reach
memory, as they should: a JS `Array`, string and `Map` is the idiom, and
their Rust bodies a SwissTable and a driftsort. The safe stratum is what's
written over them: `Option`, `Result`, the iterator adapters and the
`Iterator` trait's provided methods, `cmp`, `fmt`'s traits. That's the
stratum rust-js models by hand today, one ADR at a time.

## Decision

**A std function's body is lowered as any body is, from the MIR the
toolchain ships, into a std module of rust-js's own; a recognizer is
tidy's proved rewrite of that body into the JS idiom; a leaf of
addressable memory is refused as today.** One way a std call is lowered,
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
└─ its body reaches memory ────► refused where it's reached, as today:
                                  "rust-js does not support .. yet"
```

- **Where the JS goes**: a generated `@rust-js/std`, one module per std
  module, built once per toolchain as `@rust-js/runtime` is
  (`rust-js --runtime-module`), not lowered again into each app. A
  generic function takes dictionaries in predicate order (ADR 0049) and
  the drops of its parameters (ADR 0098); a type of the safe stratum,
  `Peekable<I>`, is the object its struct is (ADR 0020), its derived
  `Debug` and `Clone` from the derives' MIR.
- **The vocabulary**, what a lowered std body bottoms out on, is what
  rust-js lowers of any body: `Iterator::next` of a type parameter, its
  dictionary's; `FnMut::call_mut`, the closure called; `Try::branch`, `?`
  (ADR 0364); a `Drop` terminator, the value's drop; a function that
  never returns, the panic (ADR 0012); a `UbChecks` branch, off.
- **One shape per std type, never per call site.** A type's JS is
  decided once, by the type: once its body is compiled, every value of
  it is its struct, a `Peekable<I>` `{ iter, peeked }` wherever it's
  made, by generic code or by `v.iter().peekable()` in concrete code. A
  recognizer rewrites a call, never a value's shape: `$peekable`'s
  object and the struct can't both be a `Peekable`, as a value built
  where one fact was known and stepped where another was is the bug
  ADRs 0542 to 0548 each fixed once. Which shape a type has is
  `representation.rs`'s one answer, as it is for the crate's types.
- **A compiled iterator is a JS iterator at the boundary.** std's
  iterators are JS `Iterator`s today, so `for...of`, `.toArray()`, a JS
  caller and the `.d.ts` work on them; a compiled adapter's struct keeps
  that: it's given `Iterator.prototype`, its `next` the compiled `next`
  returning `{ value, done }`, as `$iterator(it, next)` gives the
  crate's own (ADR 0055). Nothing at the boundary changes.
- **Sources stay recognized, stepped by dictionary.** `slice::Iter::next`
  and `Range::next` reach memory, or are primitives: a compiled
  `Fuse<I>::next` steps `I` through the dictionary its caller gives,
  `{ next: $next }` of a `$iter`, never through an inlined slice body.
  Polymorphic optimized MIR leaves `I::next` unresolved, which is what
  the measure counts as the vocabulary.
- **A std-private trait on a type parameter is given as any bound is**
  (ADR 0049): the dictionary of the impl rustc resolves for the caller's
  type, in the caller's typing environment, so a `Pattern` of a `char`
  is `char`'s impl and of a `P: Pattern` the caller's own dictionary
  passed on; where a type meets only the blanket impl, `SpecFold`'s
  `impl<I: Iterator>`, that one, the specializations being std's
  optimizations of it, which `rustc_specialization_trait` says change
  nothing a program sees. The impl's methods are std bodies, compiled or
  recognized as any other. The `spec` count says how many bodies need
  one.
- **The primitives stay recognized**: a method whose body reaches memory
  has its JS from its type's module, as now, `Vec::push` `v.push(x)`.
  `classify` is where that's decided, as it is.
- **A recognizer of a safe body is a rewrite**: `Option::map` is
  `x === undefined ? undefined : f(x)` because that's what its body,
  lowered, folds to, not because `option.rs` says so apart from it. Each
  such recognizer is held by a corpus case that runs the idiom and the
  compiled body against native Rust; retiring one changes no answer.
- **`classify`'s refusal moves to the leaf**: a method it doesn't know is
  lowered from its body; what's refused is the memory operation in that
  body, or in one it calls, naming the std function it's in. What
  addresses memory in MIR has one owner, `pointers::mir_addresses_memory`,
  beside `raw_pointee`: the measure asks it, and the leaf's refusal will.

## Why

- **It's exact by construction**, the argument ADR 0364 made for the
  app's bodies: the order, the drops, the panics and the `Debug` of a
  `Peekable` are std's, not rust-js's reading of std's docs.
- **One mechanism replaces an enumeration**: `combinators.rs`,
  `iterators.rs` and `mir/iter.rs`, 5,300 lines, and the runtime's
  `$fuse`, `$peekable`, `$peekableClone`, `$sizeHint` model by hand what
  `core::iter::adapters` says in 7,700 lines of mostly safe Rust, with
  every direction, clone and size hint rust-js hasn't reached yet.
- **Coverage stops being linear in ADRs**: `Iterator` has 70 provided
  methods and 25 adapters; a crate's generic code calls any of them.
- **It keeps the north star where it matters**: the app's JS keeps its
  idioms, since the recognizers stay; `@rust-js/std`'s JS is a library's,
  read as `@rust-js/runtime`'s is.

## Alternatives

- **A std port in safe Rust over JS primitives, compiled by rust-js as a
  library**, as Scala.js wrote the Java library in Scala. No new lowering
  mode: a library under ADR 0100, its functions keyed by std's paths.
  Rejected: Scala.js couldn't compile the JDK; rust-js can read std. A
  port is 10 to 20 thousand lines kept against each std release, and every
  `Debug` format, panic message and edge case drifts from the real thing.
  Lowering std's MIR costs one mode once and is exact.
- **Keep enumerating**: an ADR per adapter, direction, clone and holder.
  Rejected: 42 of the last 145, with no end the list shows.
- **Monomorphize std bodies into each app**: what rustc does. Rejected
  for the reasons of ADR 0049: larger output, no JS-callable generic API,
  and a std body in the app's JS where the app wrote one line.

## Consequences

- **A second reading of MIR**: optimized, after inlining, drops
  elaborated, with `UbChecks` branches and `precondition_check` calls to
  fold away, and no THIR beside it. The tidy modules read no THIR;
  `mir.rs` and `mir/hints.rs` do, for names, spans and what a macro made,
  which an external body takes from MIR's `var_debug_info` and spans.
  That's the one lowering mode to add, and the measure says which bodies
  it must take first.
- **`none` bodies**, 27 in the registry, `to_lowercase` among them, and
  `fmt`'s builders outside it: a sysroot built once per toolchain from
  rust-src with `-Zalways-encode-mir`, as the playground's is staged
  (`wasm/build.sh`), or they stay recognized.
- **`opaque` bodies**, 16: `char`'s Unicode tables and `HashMap::entry`
  through hashbrown; each is `none` one call down, the same answer.
- **Specialization in std**, `SpecFromIter`, `SliceContains`: resolved by
  rustc for the body's types, as the measure resolves each call.
- **After ADR 0549**, which teaches a held `Fuse` its source's back
  through `$knowing`, no further adapter is modeled by hand: that stratum
  is this draft's. The draft is rebased over 0549, which touches `$fuse`,
  `$closing` and `knowing_held`.
- **What it doesn't move**: 54 of rustc's 101 known failures are
  addressable memory, and the crate corpus stops on yoke, memchr and
  powerfmt, raw pointers all. A compiled std moves the adapter and trait
  tail, not memory, which needs a decision of its own.

## Gates

1. **The measure covers the stratum it's about**: the `Iterator` trait's
   provided methods and `core::iter::adapters`' impls, beside the
   registry's data structures, with `spec` counted apart, so the count of
   safe bodies a compiled std takes over is measured, not quoted from a
   probe, and not overstated by `SpecFold`'s and `ZipImpl`'s.
2. **One external body end to end**: `Peekable::next`, lowered from std's
   optimized MIR with no THIR, into a std module an app imports, run
   against native Rust by corpus cases where `I` is a slice's iterator
   and where it's a range, each stepped through its dictionary.
3. **`classify`'s fallback**: a `Std` variant for a compiled body, its
   call emitted as a library's, and the leaf's refusal naming the std
   function it's in, asking `mir_addresses_memory`, held by
   `test/diagnostics.test.ts`.
4. **One retirement**: `$peekable`, `$peekableClone` and a held
   `Peekable`'s `Debug` (ADRs 0514, 0543) replaced by the compiled
   adapter, `held_peekable`, `peekable_drops`, `peekable_lazy` and
   `peekable_len` in the corpus unchanged, `bun run port measure`
   unchanged, their mutations caught by the compiled body's cases.
5. **The release**: `@rust-js/std` generated and published with the
   compiler, as `@rust-js/runtime` is (ADR 0120).

## What holds it

- `docs/std-coverage.txt`, each method's body kind beside whether it's
  known, held equal by `test/std-coverage.test.ts`: a toolchain upgrade's
  change shows, and the safe and memory strata stay the many.
- The architecture test: recognition may name a std function only as an
  idiom's rewrite or a primitive's method; `std_coverage.rs` asks
  recognition which std function addresses memory, `memory_intrinsic`,
  `addresses_memory`, and `pointers.rs` which MIR does,
  `mir_addresses_memory`, and spells no std name of its own.
