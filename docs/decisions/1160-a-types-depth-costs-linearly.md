# 1160. A type's depth costs its compile linearly

Status: Accepted (2026-11-15, #1821).

Case ([0262](0262-when-rust-and-js-disagree.md)): none observable. The
JS does what it did, in fewer branches. Compile time and output size are
the loop's and the reader's, so this is a dev-loop and readability
decision, not a semantic one.

## Context

The metamorphic net's rc-refcell rewrite of `nested_type_chain.rs` never
finished compiling, in either build (#1821). Reduced, it is a chain of
twenty enums, each of four variants holding the one before, deriving
`Clone` and `Debug`, cloned out of an `Rc<RefCell>`. Ten deep took 17 s
and wrote 1.1 million lines; each level multiplied both by four.

Two things grew as the variants to the power of the depth:

- **What was written.** A derived `Clone` of an enum is a conditional of
  its variants, each `{ ...e, _0: <the payload's clone> }`, and the
  payload's clone is the same conditional one level down
  (`std_impls/clones.rs`). So was a copy where the crate changes the type
  in place (`copies.rs`), a kept copy of a value React gives again
  (`copies/kept.rs`), and a derived `cmp` or `partial_cmp`
  (`ordering.rs`'s `cmp_derived`): an empty `BTreeSet` of a six-deep enum
  wrote 43,000 lines.
- **What was asked.** Questions about a type, such as whether its clone may
  panic (`clone_may_panic`), whether its `==` runs code of its own
  (`custom_eq`), or whether its order may be another's, walked its parts
  keeping only the path, so a type reached four ways was asked four
  times. And `drop_size`, which counts the drops an in-place drop
  writes, summed four to the fortieth and overflowed, so rust-js
  panicked.

## Decision

**Variants of the same fields are written once, under one test.**
`variant_groups` (copies.rs) groups a tagged enum's variants by their
fields, names and types. A variant whose fields' own definitions matter,
a cell its field's definition makes a property, stays alone.
`variants_tested` writes each group's value once, under the OR of its
variants' tests. Where one group's variants are all the variants that
have one, its test is of the fewer others, or there is none:
`deep === "Empty" ? deep : { ...deep, _0: .. }`, and a `Result<T, T>`'s
clone is `{ ...value, _0: clone(value._0) }`. The value of the other
variants is the caller's: the value itself for a copy, `0` for a
comparison. Copies, kept copies, derived clones and derived comparisons
all ask these two. There was one loop of each in four places, and each
wrote the same per-variant chain.

**A question that reaches a type's parts asks each type once.** Where the
answer is an OR, or an AND, of its parts' answers, and a part's answer is
its type's alone, the walk keeps every type it has asked, not only those
on the path. A revisit gives the neutral answer: OR's `false`, AND's
`true`. A first visit that decides it has already ended the walk, so the
answer is exact. That covers `clone_may_panic`, `custom_eq`,
`holds_identity`, `order_may`, `clone_runs_crates_code`, `clone_shows`,
`default_may_panic`, `own_ne`, `kept_may_change`, `structural_clone`,
and the two walks of what the crate changes in place in copies.rs.
`needs_clone` and `unsupported` already keep their answers, assuming
what's found inside a recursive type only as sure as the walk outside
it.

**`never_unwinds` keeps each instance's answer that doesn't depend on the
path.** It asks whether a generic function's drop, given its arguments,
never unwinds, and it walks the calls it reaches (#861). An instance it
meets again on its own path may unwind, the conservative answer while
that instance is being judged. That isn't AND's neutral, so a visited set
would have changed its answers. It kept only the path, so a call graph
with many paths to one function was judged once a path: a `Defer`'s drop
calling a chain of generic functions, each calling the next twice, took
2.2 s twenty deep and doubled with each level. Now each answer is kept
where the walk under it met nothing above it on the path, the same rule
as `needs_clone`: such an answer is the same on any path. So each
instance is judged once a walk, and every answer is the one it was. The
JS is byte for byte the same, and so are its source maps.

**`drop_size` sums saturating**, as it's only compared with 8.

## Consequences

- The reduced program compiles in 0.14 s twenty deep and 0.21 s forty
  deep, its JS 216 lines ten deep where it was 1.1 million. A chain of
  every derive (`Clone`, `Debug`, `PartialEq`, `Eq`, `Hash`,
  `PartialOrd`, `Ord`, `Default`) beside one whose payload has a `Drop`,
  cloned out of an `Rc<RefCell>`, compared, hashed, sorted, defaulted
  and dropped, compiles in 0.3 s twenty deep and 1.9 s eighty deep,
  in both builds.
- `test/type-depth.test.ts` holds this by counting lines, which are
  deterministic where a run's time isn't: depth 10, 20 and 40 in each
  build, and each doubling of the depth must give less than eight times
  the lines. It's under eight, not two, because the inline comparison's
  paths, `a._0._0`, grow with the depth, so the text is quadratic. On the
  compiler before this, twenty deep hit the test's 60 s compile timeout.
- `test/corpus/enum_variants_of_the_same_fields_copied_and_compared_once.rs`
  holds the grouping against native Rust in both builds: an enum whose
  `Circle` and `Square` share fields, whose `Line` has its own and whose
  `Dot` has none, cloned out of a `RefCell`, changed, and compared with
  `partial_cmp`, a NaN's too, and `==`; and a nest of four variants
  sorted and compared. Three corpus snapshots read shorter: a
  `Result<T, T>`'s clone, as one copy.
  Readability's counts are blessed by the new case's alone, its one
  `$iter`, 12 renamed `x$1` and 6 role fallbacks; the three that moved
  are 33 lines shorter.
- Left, as readability (#1879): a derived comparison of a deep type is still
  written inline, one level a line, where a function per type, as
  `Debug`'s `c3Debug_fmt`, would read as hand-written code. A drop glue's
  variants of the same fields are each their own `if`. Both are linear.
- `test/type-depth.test.ts` holds `never_unwinds` too: that chain forty
  deep, which the compiler before this never finished, must compile within
  its 60 s timeout. It compiles in 0.1 s.
- Mutations: `never-unwinds-asked-each-way` (`lower/analysis/unwinding.ts`),
  `variant-groups-never-merged` and
  `variants-tested-by-the-group` (`lower/copies.ts`),
  `drop-size-overflows` (`lower/drops.ts`),
  `clone-may-panic-asked-each-way` (`lower/std_impls/in_turn.ts`) and
  `custom-eq-asked-each-way` (`lower/std_impls.ts`).
