# 0902. What never unwinds is read from MIR

Status: Accepted, under the user's delegation of 2026-10-25 ("proceed as
your recommendation"); open to their review. The review's P2: whether a
node can unwind, computed once from MIR and carried on the JS tree.
Extends [0901](0901-a-print-throws-only-where-its-text-does.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): no difference. A `try`
only goes where nothing in it can throw; what runs, and in what order, is
the same.

## Context

After ADR 0901, a print of numbers and text throws only where its text
does. A call of the crate's own function was still one that may throw.
So `loudDrop_drop(b)`, a `Drop` that only prints, put the drops after
it in a `try`/`finally`, even though nothing in it can panic.
`may_throw` also took a template or an array that held any call as
throwing, without asking its parts.

## Decision

- **`never_unwinding` (`analysis/unwinding.rs`) finds, from all the
  crate's non-generic functions down, those whose every block can't
  panic.** A block's terminator must be a jump, a test, a return, or a
  call of another such function, or of std's `_print`/`_eprint`, its
  `Arguments`, or an `Argument`'s `Display`/`Debug` of a number, a
  `bool`, a `char` or text. Never an assert (an index's bound or a
  divisor's), a drop, nor what's unreachable. Overflow checks are off,
  as release Rust's (ADR 0011). Cleanup blocks don't count.
- **A reference to one is marked `Known::NeverThrows`** where every
  reference is made (`fn_ref`). `may_throw` of a call of it is its
  arguments'.
- **`may_throw` of a template, an array, a binary operation or a
  condition is its parts'**, as `has_effects` reads them:
  `` `${total(x)}` `` throws where `total(x)` does.
- What it exposed, each fixed:
  - **A temporary made by the program's code is made before a call
    that unwinds elsewhere**, as one that may throw is
    (`may_throw_first`). `slot.pending.insert(made(5))` dropped the old
    value before `made(5)` ran, printing `drop 4` before `made 5`
    (`option_destructors.rs`).
  - **`stmts_alike` ignores what's known of each part**, as it does
    where each is written: the same JS, one copy of it marked.
  - **A temporary's new name isn't given to a closure's own parameter of
    its name** (`js::outside_shadows`). Renaming `_2` to `value` renamed
    `(_2) => _2[0]` too, so the closure read the outer value
    (`temporaries_taken_apart.rs`).

## Consequences

- 52 snapshots change: 43 `finally`s and 147 `catch`es go, 1,718
  lines fewer. One reads worse, `given_last_unwinds.js` (+12 lines): a
  drop inside its loop no longer needs its own `try`, so the scope
  forms inside the loop and no longer wraps it. That's correct, and
  left for a `Run::covers` that takes a `try`/`finally` in.
- The readability baseline is blessed: each shape's share grows 1 to
  2% as the lines around it go (`$iter` 84.8 to 86.3, `renamed x$1`
  727.9 to 731.5); no construct is new.
- `never_unwinding_calls.rs` holds both sides against native Rust: a
  print-only function needs no `try`, and an index past the end or a
  panicking `Display` still drops what's held.
- Mutations: `assert-steps-on`, `user-display-shown-whole`,
  `never-unwinding-unmarked`, `template-may-throw-whole`,
  `program-code-pending-unflushed`, `shadowed-temporary-renamed`.

## Amendment: a closure never unwinds where its MIR can't panic, as a function doesn't (2026-11-04, #701)

Case ([0262](0262-when-rust-and-js-disagree.md)): no difference. What runs,
and in what order, is the same; a collect only leaves out the closing a
panic would need.

`never_unwinding` took only functions, so every closure stage counted as
one that may panic in `steps_may_panic` (ADR 1089), and a collect of items
with a destructor through a closure that can't panic was
`$collectDropping($closing($iter(v), ..).map(f), drop)` where `v.map(f)`
does what Rust does.

- **A closure is a candidate, as a function is**, its MIR's every block
  read by the same `steps_on`. `steps_may_panic` already asked
  `never_unwinding` of each closure of an iterator's type; nothing else
  decides it.
- **Its captures need nothing of their own.** A closure reads what it
  captured as its fields, so a call, a drop or an assert of a capture is a
  block `steps_on` refuses. Dropping the closure itself, its captures, is
  no step of it: where an adapter drops one as it's stepped, a `chain()`'s
  exhausted side, that's the adapter's work, which `steps_may_panic` asks
  recognition's `stepping_work` of (ADR 1089, amended).
- **A closure of a generic function isn't one**, as the function isn't:
  its MIR reads the function's type parameters, whose values come through
  dictionaries, so it's held to the rule its parent is. No JS shows it
  today: a closure's type holds its function's type parameters, and
  `steps_may_panic` takes an iterator whose type holds one as one that may
  panic, so `held`'s collect stays `$collectDropping`; it keeps
  `never_unwinding` true of what it holds, whoever asks next. A coroutine,
  an `async` block's, isn't one either: no stage is one.
- Overflow checks stay off (ADR 0011): `|n| n + offset` never panics,
  `|n| table[n]` and `|n| 6 / n` may.

What it exposed: a `filter()`'s and a `take_while()`'s drop of the item
its closure rejects, a `fuse()`'s or a `chain()`'s of the part it has
exhausted, and a `flat_map()`'s of its inner iterator were counted only
through their closure. They're `stepping_work`'s now (ADR 1089, amended),
which landed first, so this changes what may panic only where a closure
can't.

`closure_stages_that_never_unwind.rs` holds both sides against native
Rust, `closure_stages_that_never_unwind_past_adapters.rs` the adapters'
own drops behind closures that can't panic. Mutations:
`closure-never-candidate` (`lower/analysis/unwinding.ts`),
`filter-drops-no-items` and `fuse-drops-nothing`
(`lower/recognition/iterators.ts`); a closure of a generic function taken
for a candidate is no mutation, as nothing shows it.

One limit stayed: `steps_on` refused each `drop` terminator, a moved-out
temporary's too, so a function or a closure that builds a value of a type
with a destructor from another, `Once { held: D(n), left: 1 }`, was taken
for one that may unwind; it steps on since #717 (below).

## Amendment (2026-11-05): a drop that drops nothing (#717)

Case ([0262](0262-when-rust-and-js-disagree.md)): none; the JS read worse.
`steps_on` refused each `Drop` terminator, so a function that builds a
value of a type with a destructor from a temporary it gives on, or
drops what JS drops nothing of, was taken for one that may unwind, and its
callers wrapped each call of it, `let tmp; try { tmp = many(10); } catch
(error) { throw error; }`, or collected through it by `$collectDropping`.

**A `Drop` that drops nothing steps on**: of a type whose drop JS runs
nothing of, the analysis's `runs` (`ItemDrops`, the lowering's answer of
the same), or of what's surely moved out by then, a temporary given on,
as rustc's `MaybeInitializedPlaces` says before it. `mir::dead_drops` is
the one answer, beside the lowering's elaboration, whose dataflow
(`maybe_inits`) it shares, so both ask rustc's the same way. A body whose
locals' facts aren't the crate's, a `const fn`'s read for a pattern,
runs no dataflow, its drops of what's there taken to drop it; one with no
other `Drop` runs none either.

`test/corpus/drops_of_nothing_never_unwind.rs` checks both, a `String`
given and dropped and a value given on, beside a value with a destructor,
whose calls take no `try`. Otherwise what changes is the JS alone: 15 corpus
cases lose the `try` around such calls, `const _two = [undefined, many(10)];`, the user's decision to
drop them as no-ops. The mutations are `drop-never-steps-on`
(`lower/analysis/unwinding.ts`) and `moved-out-drops-there` and
`quiet-drops-there` (`lower/mir/drops.ts`).

## Amendment: a temporary that runs the program's code is no longer made first (#781)

`may_throw_first` also held a temporary made by the program's code that
never unwinds, `made(5)`, as made first, so `slot.pending.insert(made(5))`
made the value before `insert` dropped what the slot held. Since #765
(`Option::insert` makes its value first where it's lowered) and #723's
shared cleanups (78372e8ba), nothing reaches that answer: its mutation,
`program-code-pending-unflushed`, survived the Nightly of f3f23cd67, and
the whole corpus, the snapshots, the rest of check:fast's suite and
probes of `Option::insert`, `HashMap::insert`, `Option::replace`, an
index's and a variable's assignment of a value `made(n)` gives print the
same JS without it. `may_throw_first` is `may_throw` again, and the
mutation goes with the term. `option_destructors.rs` and
`never_unwinding_calls.rs` still hold the order against native Rust.
