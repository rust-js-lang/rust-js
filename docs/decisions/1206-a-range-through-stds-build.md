# 1206. A range through std's build

Date: 2026-11-25.

Status: accepted, steps S0 to S3 (Campaign 5 step 3b, #1589). Amends
[1202](1202-one-catalog-of-the-runtimes-models.md).

## Context

The runtime's range helpers, `$rangeNext`, `$rangeNth`, `$rangeNthBack`
and the others ADR 1202's catalog says are `Unproven(Ranges)`, are rust-js's
models of std's `Range` and `RangeFrom` iterators. The twin net proves a
model by a case whose plain build imports it while its std build, the
differential build, `--std-iterators`, runs std's own compiled body there
instead. For ranges, std's build couldn't compile those bodies. #1589 named
five reasons: their literal panics, their specialized private trait
`RangeIteratorImpl`, the `PartialOrd` of their bounds, each integer's
macro-written `Step`, and the recognizers treating every range as theirs.

Measured on this toolchain by giving std's build `Range`'s rows:

- A range already is std's struct in the plain build. `a..b` is
  `{ start, end }` and `a..` is `{ start }`, std's fields, so only the
  steps are rust-js's. `RangeInclusive` differs: std's holds `exhausted`,
  which rust-js's doesn't. It stays out of this decision.
- `.expect(msg)` in std's bodies is a call of its private
  `core::option::expect_failed(msg)`, `inline(never)`, which nothing
  placed: "a crate given std's bodies has no module for
  `core::option::expect_failed`".
- `Step` wasn't a trait given dictionaries, so `Range<A>`'s body had no
  `A: Step`, and none of `PartialOrd`, its supertrait.
- `RangeIteratorImpl`'s impl for a `T: TrustedStep` specializes the
  generic one, and no fact chose it.

## Decision

Std's build compiles `Range`'s and `RangeFrom`'s `Iterator` and
`DoubleEndedIterator`, std's own `Step` of each integer and `char`, and
`RangeIteratorImpl`'s two impls, in steps:

- **S0.** `expect_failed(msg)` is `panic_display(&msg)` behind its
  `#[track_caller]`, its payload a `String` (ADR 0660). It's recognized as
  that, `Std::PanicDisplay`, by one predicate, `std_items::is_expect_failed`.
  `classify_fn` asks it, and so does compiled std's walk, which compiles
  every private callee of a compiled body, as no program can name one:
  this one a recognizer answers, as `io`'s answered functions are. What
  `panic_display` shows is its argument, `x: &T` or `msg: &str`, seen
  through its reference, `tys[0]`, the same rule for both. A consumer's
  copy of a library's `.expect(..)` lowers from the library's own body,
  `$unwrap`, so only std's build reaches it.
- **S1.** Std's own `Step` of each integer and `char` is compiled into
  `@rust-js/std`, the dictionary a range's compiled body steps its bounds
  by. A runtime model of `Step` would leave the proof resting on a model.
  The catalog's one table of rows (`compiled_std.rs`) keys a row by what its
  methods are of: `Of::Struct(StdPath)`, std's struct as before, or
  `Of::Primitives`, each std impl of the row's trait for an integer or
  `char`, the impls rustc keeps under a primitive's simplified type. That
  extends the table's key; it adds no second table. `Step`'s nine methods
  are its rows, and its four defaults are `COMPILED_STD_PROVIDED`'s, as
  `char` writes neither `forward` nor `backward`. `Step` is operational
  (`recognition::operational`), holds its `Clone` and `PartialOrd`
  (`holds_supertraits`), and its dictionary holds every method, each its
  impl's or the default (`in_std_dictionary`). It's unstable, so no program
  bounds it, and the plain build's output doesn't move; std's build compiles
  all thirteen without a refusal.
- **S2.** Std's `TrustedStep` is a fact of a type, `TypeFact::TrustedStep`,
  which `specialized_on` names as `RangeIteratorImpl`'s, so the fact picks
  the impl rustc picks, a `TrustedStep`'s, which every `Step` std has is.
  Its caller gives it as it gives `TrustedLen` (`relay`). A `T: TrustedStep`
  bound is given as the `T: Step` it implies (`traits::bounds_of`), as
  `I: DoubleEndedIterator` is given as its `I: Iterator` (ADR 0488): the
  specialized impl's body steps by its caller's `Step`.
- **S3.** `Range`'s `Iterator`, `DoubleEndedIterator` and `Clone` are
  `MEANING_ONLY` rows: in the build of std's meaning, `--std-iterators`,
  `a..b` is std's struct, `$stdIterator({ start, end }, steps)`, stepped
  by std's compiled bodies, `rangeIterator_next(r, u32Step, true)`, `u32Step`
  std's own `Step` of a `u32`, `true` its `TrustedStep`. The plain build's
  output doesn't move: there, as before, a range is the recognizers'.

  Two authorities would step one range otherwise: the recognizers answered
  every range question by `range_kind`, so the build of std's meaning
  stepped std's struct by `$rangeNext` beside std's bodies. The question is
  split at its owner (recognition/types.rs). `range_kind` says what a range
  is, its shape: indexing `v[a..b]`, `{:?}`, `==`, hashing, serializing,
  `contains`. `helper_stepped` says whether the recognizers' helpers step
  it: `range_kind`'s range, but one this build holds as std's struct
  (`is_compiled_std_type`). It gives a `RangeSteps`, which only it makes,
  its field private to the module, and the step helpers are its
  (`RangeSteps::step`): a step can't be asked of a bare `RangeKind`.
  `step_helper`, the one table, is read otherwise only where a step rust-js
  made is read back from its JS, `iterators::stepped_range` of a `$iterator(r, next)`
  and `range_from_stepper`. Every caller that steps, counts, reverses,
  lends or makes a dictionary of a range asks `helper_stepped`, 36 of them;
  28 shape questions stay on `range_kind`. test/architecture-iterators.test.ts
  holds who reads `step_helper` and makes a `RangeSteps`.

  What makes a range makes std's struct where one is held: the aggregate,
  as before, and `Default`, its bounds' defaults held the same way; one maker,
  `std_struct_held`, which MIR's aggregate and std's makers call too, a
  clone keeping the steps of the one it clones. A clone is std's compiled
  `Clone`'s. Deserializing one, whose serde form makes plain fields, is
  refused in the build of std's meaning alone (ADR 0262's last case, of a
  test build). A range's `{:?}` is rust-js's own of its fields, whichever
  steps it, so a `{:?}` of one never makes it std's struct in the plain
  build (`note_debug_shown`, #1401), asked of the type by `range_kind_of`.

  `RangeFrom` (`$rangeFromNext`) follows as S4, and `RangeInclusive` stays
  out: std's holds `exhausted`, which rust-js's doesn't.

  Five wrong answers of the build of std's meaning, found before S3 landed,
  were each a site asking the wrong question (ADR 1085's three of a kind),
  so each question now has one owner that a site can't get around:

  - **Which range question.** A range's shape, `range_kind`, is what its
    bounds, `{:?}`, slicing, hashing and whether a count is the program's
    ask; `helper_stepped` only what steps a range or holds its JS form.
    lengths.rs asked `helper_stepped`, so a count of a range std's bodies
    step wasn't the program's, and a generic collect of `usize::MAX / 2`
    items checked no room. test/architecture-iterators.test.ts names each
    module that may ask `helper_stepped`, with what it steps.
  - **Which function makes a struct where it's no iterator.**
    `makes_no_iterator` takes the call, its callee and arguments, and asks
    the function it resolves to, so a trait's method is asked by its
    impl's own, `Range::<u8>::default()`'s, never by the trait's item that
    no catalog names. MIR's dispatch and the analysis both asked the item.
    A maker makes the struct of its parts: one given the struct it gives
    back, std's compiled `clone`, keeps the steps of the one it's given.
  - **Which derives are compiled.** `Range` had no rows for its derived
    `PartialEq`, `Hash` and `Default`, so a range std's bodies made and a
    plain one compared and hashed apart. `bun run std` now refuses to keep
    the catalogue while std derives a `Clone`, `PartialEq`, `PartialOrd`,
    `Ord`, `Hash` or `Default` method of a struct the catalog compiles that
    no row names (`rows::derives_unnamed`), but each `DERIVES_LEFT_OUT`
    says why: today the `std::net` addresses' orderings, as comparing them
    is refused. The other ranges are no compiled struct, so none of theirs
    is asked until one is.

## Consequences

- The twin net pits `$rangeNext`, `$rangeNth`, `$rangeNthBack` and the other
  helpers a case's plain build imports and its std build no longer does;
  the metamorphic net's std build runs the corpus's ranges against native.
- A held compiled struct's `nth(n)` in either build is the default over its
  `next`, `$lent(it).drop(n).take(1)`, not std's own `nth`, a `Peekable`'s
  as a range's: results agree, as its `next` is std's, but std's own `nth`
  bodies aren't what runs. Found here, not changed here.

