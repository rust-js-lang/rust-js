# 1004. The mutations surviving at main's head

Status: Accepted. Part of #382.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. No JS a program
gets changes. This changes what the mutations check, and removes code
nothing reaches.

## Context

Main's nightly failed: 14 mutations survived at its head, and main's checks found more. Each was
probed by reblessing the whole corpus under it, then checking the forms
corpus and the mutation's own tests.

## Decision

- `by-ref-back-lost` and `impl-trait-hides-held-into-iter` change only
  how the JS reads. Under each, the JS is still right:
  - `$lent(Iterator.from(it))` is `it`, since `Iterator.from` gives an
    iterator back.
  - A JS iterator with dictionaries made in the function, in place of
    `J` given as it is.

  So each now checks its case's snapshot (`snapshots: true`).
- `map-length-unknown` fails two cases that ask a map's length,
  `map_stepped_from_its_back.rs` and `owned_maps_asked_their_length.rs`.
  Its tests now name them.
- `rev-of-own-back-reversed`: `rev_items`' arms of an `Iterator.from(x)`
  receiver are unreached. With either arm panicking, the corpus passes.
  The last arm is now what steps any other receiver from its back,
  `as_double_ended`'s, which refuses what it can't make. Nothing falls
  back to an array of the items read from the front and reversed, which
  would be a wrong answer.
- `impl-bound-held-by-itself`: whether an impl holds an `IntoIter` is
  now asked of each function or impl alone, not of all of an impl's
  methods. Nothing's JS depended on the impl-wide answer.
  `test/corpus/into_iter_held_by_a_sibling.rs` checks the shape it was
  for, an impl some of whose methods hold `A::IntoIter`, against native
  Rust.

- Five are caught by tests that reach them again:
  - `written-break-not-first`: React's `addValueToProperties`, whose
    `break 'desc` ends a case, in the port's performance-track test, as
    6ca2f9688 found too. The probes had missed it: the corpus doesn't
    reach it.
  - `dictionary-cache-runs-code`: the dictionary test's cycle of
    dictionaries alone now holds a generic impl's cache and a dictionary
    the other module reads. Under the mutation that cycle is refused.
  - `moved-into-owner-unheld`: `iterator_moved_into_a_holder.rs`, a
    `take` moved into `wrap(..)`'s `Holder` and stepped past its limit.
    Under the mutation the rest drops before the `println!`.
  - `null-nones-calls-unwritten`: lowering-nullable's nullable
    function's result, 0faaa845d's. A new nullable-reads test,
    `halved_lazy`, also reaches it: a call's `Option` kept in a variable
    that a nullable field alone reads, whose `?? null` is where it's made.
  - `unread-ref-held-at-write`: `if_let_mut_else_assign.rs`'s `named`.
    Under the mutation it's `const f = found; f.push(v)`, so the mutation
    checks snapshots.
- One goes with the code it guarded, as nothing's JS depends on it:
  - Tidy's "stored again" pass. The places that store a nullable value
    store it again themselves, so the corpus, the examples, the React
    tests and every test that mentions a nullable place are the same
    without it.

- Of those main's checks found:
  - `double-ended-of-a-js-iterator-again`: `as_double_ended` unwraps a
    stepped iterator, `$iterator(range, $rangeNext)`, by `stepped_value`,
    the one reader of that form. ADR 1002's `range_object` did the same
    for ranges alone, a second way. It's gone, and `as_exact_size`'s
    range rule reads `stepped_value` too, so the mutation is caught again.
    `stepped-range-restepped` is now `as_exact_size`'s.
  - `filter-made-here-backless` fails four cases a filter made in place
    and stepped from its back is in. Its tests now name them.
  - `enumerate-explicit-too`: an explicit adapter is never one compiled
    where it's held (ADR 0871), which the code asking `explicit_adapter`
    relies on, as 003efa32e removed what read one. That's now checked in
    a debug build, so a held `Enumerate` that's explicit too panics.
  - `negation-spilled`: the JSX lowering's reading of `!x` as reading the
    same as `x` changes no element. Every JSX and React test is the same
    without it, so it goes. A new test checks that a prop of `!hidden`
    before a later child's statements stays in place.

  - `settle-captured-elements`: `object_made_past_a_drop.rs`, an
    object made where it's used, reading a reference, past a drop. Under
    the mutation it's settled before the drop, `const result = ..`, so
    the mutation checks snapshots.
  - `chains-empty-if-kept`: none of the chains stage's passes empties an
    `if` now, as the names and written stages clear those of `if let`s
    whose bindings are only read. Its "never taken" pass goes; the
    stage's `NO_EMPTY_IF` contract, checked in a debug build, stays.
  - `written-order-goes-round-kept`: of a loop body's `if` whose branch
    is one statement, no test's JS depends on whether the rest goes
    round: a later stage writes the program's order either way. As a
    lone exit is one statement too, the rule is "more than one
    statement", and its three mutations are one,
    `written-order-one-exit-turned`.

## Consequences

Those eighteen mutations are caught or removed. The rest are listed on
#382 until they are too.

## Amendment: nightly 40752714381's iterator survivors

The nightly of 6cce16f37 found more.
- `zip-not-cloned-apart`: a zip cloned is std's struct, cloned by its
  compiled `Zip::clone` (9dc8123b8). The cloned-apart form of one, a
  `makeZip` factory, was a second way. It goes, `Parted::Zip` with it, and
  `cloned_chains_and_cycles.js` is shorter.
- `zip-side-array-uncloned`: a cloned-apart chain's other side given as a
  `Vec` is never reached, as such a chain is std's struct too. The arm
  goes; such a side is refused, not made.
- `rev-length-unknown` and `generic-source-unclosed` name the cases that
  fail under them.
- `copied-back-uncompiled` is caught by `associated_double_ended`, now in
  the forms test's default list.
- `lent-iterator-unheld`, `generic-iterator-unheld` and
  `iter-method-type-unrevealed` change only how the JS reads now, so they
  check snapshots.
- `maker-boxed-closure-lazy`: every `pub` thread-local is made as its
  module loads (ADR 1006). So the case gains a private one, which the rule
  still decides.

Five more are the runtime's own closes of a source at its end or its
limit: `closing-not-dropped-when-done`, `closing-nth-done-kept`,
`take-try-fold-limit-kept`, `take-try-fold-limit-left-open` and
`extreme-by-empty-left-open`. No program can show them. Every consumer
closes what it steps where it's done with it: `$Steps`' own
`[Symbol.iterator]`, `fold` and `count`, and `$taking`'s tryFold's
callers. Neither build's corpus changes under any of them, nor do probes
exhausting a map whose closure holds a destructor through JS's helpers.

Decided (agent-opal, under the user's goal "the choice is your
recommendation"): the closes stay, and their mutations go. Case
([0262](0262-when-rust-and-js-disagree.md)): none, as nothing observable
differs. A close done twice drops nothing twice, as `$closing` closes
once. A close missing, should a consumer ever skip its own, drops nothing
at all, a wrong answer. So the duplicate is kept as the safe one of the
two ways, and the rule's owner is still the consumer. A change that makes
a consumer rely on one of these must add a case that shows it, with a
mutation.

## Amendment: the nightly's survivors outside the iterators

Of the 23 left in nightly 40752714381, I took eleven, and m6's agent the
twelve in tidy. Each was probed by the corpus with its snapshots
compared, its own test file whole, and, where those pass, every
lowering, port, React and JSX test.
- Snapshots catch `else-slot-unsaid`, `decisions-loop-exit-kept`,
  `else-if-unelsed` and `never-falsy-duration`, which now check them.
- A test elsewhere catches each of these, now named by it:
  - `else-if-links-unheld`: lowering-written-branches' "an if the program
    wrote before a return keeps its body inside".
  - `flattened-made-spread-literal`: jsx-props' "JSX gives a flattened
    struct's fields as a component's own props".
  - `transparent-field-projected`: `catch_unwind_assert_drops`, where the
    compiler crashes under it.
  - `library-params-never-drop`: crates' "two crates: rc_per_type".
- `passed-on-without-fact` didn't build, as `Forwarding` gained fields; its
  replacement gives them.
- Two go with the code they guarded, as nothing reaches it:
  - `applied.rs`'s inlining of a closure that only throws, of a
    thread-local's `with(|e| js::throw(e))`, which no form makes since
    ADR 1006. `applied-throw-after-return` goes with it.
  - `Expr::unary`'s `!null` as `true`, as an `Option`'s test is `== null`
    now (4b30cd7cd), so no `!null` is made.

## Amendment: nightly 40783711256's survivors outside tidy (agent-tdyk, 2026-10-29)

Each was rechecked at main's head, then built and run over the whole
corpus, its JS compared case by case with the compiler's as it is, and
over the lowering, snapshot, JSX, port and compiler suites.

- **Caught by a corpus case their tests didn't name**, which they now name:
  `scopes-twice-allowed` by `atomics.rs`, whose JS declares `previous`
  twice in one block, a `SyntaxError`; `mir-tidy-named-temporaries-off`,
  inconclusive as it ran the whole corpus, by `tested_options.rs`'s
  snapshot; and, as they change only how the JS reads, by snapshots:
  `collected-runtime-arrayed` (`cell_deque_char_methods.rs`, whose
  `$decodeUtf16(..)` it lists first), `mir-return-written-by-storage`
  (`store_get_mut.rs`), `let-else-copy-kept` (`non_null.rs`),
  `mir-tidy-let-else-of-written-returns` (`local_consts.rs`) and
  `mir-tried-named-value` (`drop_scopes.rs`).
- **`boxed-and-refused` by a new case**, `and_of_a_generic_option.rs`:
  `and` of a type parameter's `Option`, given `Some(None)` among others,
  against native Rust. `and_value_drops.rs` no longer makes one.
- **`last-dropping-left-open`** is a runtime close of the kind the first
  amendment decided: `$lastDropping`'s `it.return?.()` after a `for … of`
  that ran to its end. Each of the eight cases that call `$lastDropping`
  runs as native Rust does with it gone, so no program shows it. Decided
  the same way: the close stays, the mutation goes.

Left open, as no rust-js test reaches them and React's port, which they
were written for, no longer builds at its last commit with main's
compiler (`react-reconciler`'s `pub` thread-locals in an import cycle,
ADR 0037, amended), so it can't show whether they're reached:
`js-object-deref-lowered`, `truth-implies-nothing` and
`std-macro-pointee-copied`. And `switch-copied-returns-kept`, which React's
port does reach (validateProperty's `switch` in an `if`), but no corpus
case yet.

## Amendment: main's check at d82b8463a's tidy survivors (agent-tdyk, 2026-10-29)

Four of `switches.rs` and `jumps/continues.rs` survive from 6ed14b332 on,
before this session's commits, and were swept as above. React's port was
built besides, every package with the compiler as it is and with each.

- `loop-break-switch-labeled` changes `match_breaks_its_loop.rs`'s JS,
  whose snapshot it now checks: the labeled loop's path, which
  `blocks_breaking_loops` (98c8bb7b1) left no `loop:` to give, still reads
  the `switch` apart.
- `switch-case-ending-break-kept`: the first `ending_breaks` of
  `switches`, of each case that doesn't fall, changes no corpus case's JS
  and none of React's port's. It goes with its mutation; the last case's
  stays.
- Left open: `jumps-into-arms-that-leave-their-own-way`, which React's port
  reaches (CSSPropertyOperations, ReactFiberHydrationDiffs) and no corpus
  case does yet; and `switch-failing-guard-goes-after`, which nothing
  reaches, but whose filter keeps a failed guard from running the
  statements of the arm others fall to: a wrong answer were it reached, so
  it isn't removed without a case.

