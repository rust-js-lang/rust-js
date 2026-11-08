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

## Amendment: a generic `Drop` impl is judged for each use (#861, 2026-11-06)

Case ([0262](0262-when-rust-and-js-disagree.md)): no difference. What runs,
and in what order, is the same; a drop that can't panic is a `finally`'s
again, where #847 made each that may a `catch`'s and the drop after the
`try` (ADR 0660, amended).

`never_unwinding` takes no function of type parameters, so a generic `Drop`
impl, the guide's `impl<F: FnMut()> Drop for Defer<F>`, was one that may
unwind whatever it was given, and `Defer(|| DEPTH.set(previous))` dropped
after the `try` and in a `$cleanup`.

**Each use asks of the impl as it's given its arguments**
(`never_unwinds_given`, `analysis/unwinding.rs`, which the lowering's
`drop_never_unwinds` asks and remembers for each function): its MIR, the
arguments filled in, steps on as one of no type parameters must, read by the
same `steps_on`. A call of a trait's method is what it resolves to in the
caller's typing environment, a closure's `call_mut` the closure, which
never unwinds where `never_unwinding` holds it; a call of another generic
function is judged so in turn, one reached again as one that may. A `Drop`
in it is of what drops nothing (`mir::dead_drops`) or of what the lowering's
`drop_may_panic` says can't panic. The drop's call is marked
`Known::NeverThrows` where it's made (`drops/parts.rs`), as a reference to
one of no type parameters is, and `drop_may_panic` asks the same of each
type's own `Drop`. A dictionary's `$drop` of a generic type whose `Drop`
only prints, `console.log(`drop ${wrapper[1]}`)`, needs no `try` for its
fields' drops after it.

`test/corpus/generic_drops_judged_per_use.rs` holds both sides against
native Rust: `Defer` of a closure that can't panic, and of one that may (an
index past the end); a drop calling a generic function given a value whose
drop only prints; and a drop of a type parameter's value, of a number and of
one whose drop may panic. Ten other corpus snapshots read better, each a
`finally` or no `try` where a generic `Drop` only prints, and the guide's
`Defer` is `try { return work(); } finally { depth = previous; }`.
Mutations: `instance-drops-may-panic`, `instance-calls-unresolved`
(`lower/analysis/unwinding.ts`), `generic-drop-judged-whole`
(`lower/drops.ts`) and `generic-drop-call-unmarked` (`lower/drops/parts.ts`).

## Amendment: a `Cell`'s set of what drops nothing, its replace and an `Option`'s take (#861, 2026-11-06)

Case ([0262](0262-when-rust-and-js-disagree.md)): no difference, as above.

#847 took a `Cell`'s or a thread-local `Cell`'s `set` or `replace` for one
that never panics only of a value `is_trivially_pure_clone_copy`, a number
or a `bool`. React's `CURRENT_DEBUG_INFO.set(self.0)`, an `Option` of a JS
object, and a restore of an `Option<Rc<Fiber>>`, `CURRENT.set(self.0.take())`,
were ones that may, and their guards dropped after the `try` and in a
`$cleanup`.

- **A `set` never panics where JS drops nothing of the value it replaces**
  (`std_never_unwinds`' `drops`, the analysis's `runs` of the body, the
  lowering's `drops` of an instance): an `Option` of a reference, of a JS
  object or of an `Rc` no count is kept of. One of a value with a
  destructor, a `Loud`'s or a counted `Rc`'s, may, as that drop may.
- **A `replace` never does**: it gives the old value back, which its
  caller drops where MIR says.
- **An `Option`'s `take` never does.**

`test/corpus/cell_restores_never_unwind.rs` holds both sides against native
Rust: restores of an `Option<&Fiber>`, an `Option<Rc<Fiber>>` taken, a `u32`
replaced and a `&Cell` given, in a `finally`; an `Option<Loud>` and an
`Option<Rc<Loud>>`, whose old values have destructors, after the `try` and
in a `$cleanup`. `refused_drops_and_displays.js` reads better, a `take()`
of the crate's that never unwinds before the `try`. The readability
baseline is blessed with both amendments' cases: like for like, `renamed
x$1`'s share grew from 549.9 to 550.1 in 10,000 lines as the generic
`Drop`s' `try`s went, no rename new, and with the cases it's 549.4. Mutations:
`cell-set-of-copies-only`, `cell-set-of-what-drops`,
`cell-replace-may-panic` and `option-take-may-panic`
(`lower/analysis/unwinding.ts`).

## Amendment: an iterator's close of what drops nothing that may panic (#861, 2026-11-07)

Case ([0262](0262-when-rust-and-js-disagree.md)): no difference, as above.

An iterator dropped by closing it, `it.return()` (ADR 0422), was a call
that may throw whatever it held, so each scope that held one, a `Peekable`
of items whose drop only prints say, dropped it after each `try` and in a
`$cleanup`: 811 of the corpus's 1,968 `$cleanup`s.

**A close never throws where nothing it may hold has a drop that may
panic** (`close_may_panic`, `drops/parts.rs`): the iterator's own drop
(`drop_may_panic`), and, through each std adapter it's made of, each one's
`Item`, which a `Peekable` holds peeked and its type's arguments don't
name, and a flatten's inner iterator. The close's call is then marked
`Known::NeverThrows`, so it's a `finally`'s.

`test/corpus/iterator_closes_of_quiet_items.rs` holds both sides against
native Rust in both builds: a `Peekable` of quiet items, in a `finally`;
of `Loud`s that may panic, of the crate's own iterator of them, whose
`Item` its type doesn't name, and a `map()` of that, each after the `try`
and in a `$cleanup`, a peeked `Loud`'s panic dropping what's after it. 104
corpus snapshots read better, 4,300 lines fewer. The readability baseline
is blessed: `while (true)` 8.7 to 8.1 and `renamed x$1` 549.4 to 538 in
10,000 lines, as the `try`s each step held went; `$iter` 94.4 to 95.8 and
`Iterator.from` 8.7 to 8.8 grow only as the lines around them go, none
new. Mutations:
`close-of-quiet-items-may-throw`, `close-item-unasked` and
`close-adapters-unwalked` (`lower/drops/parts.ts`).

## Amendment: a library's type's own `Drop` may panic (#1031)

Case ([0262](0262-when-rust-and-js-disagree.md)): R. Results changed:
`drop_may_panic` read a type's own destructor only of the crate's own
types, `is_local()`, and took a library's as std's, which may panic only
by its type arguments: a consumer's own collect of a library's `D` whose
drop panics left out the cleanup that drops the rest and what was
collected. A library's type is read as the crate's is; its destructor is
one this crate never knows never unwinds, so it may. The pinned React
port regenerates as before. Mutation `library-drop-never-panics`
(`lower/drops.ts`), by the pair test `library_drops_its_consumer_gives`.
