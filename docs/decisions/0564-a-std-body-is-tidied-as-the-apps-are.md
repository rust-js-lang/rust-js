# 0564. A std body's JS is tidied as the app's is: decided tests, once-read temporaries, shared tails

Status: Accepted. Extends [0364](0364-mir-lowering.md) and
[0561](0561-std-bodies.md). Extended by [0743](0743-a-part-of-a-chain-a-test-decides-is-left-out.md): a part of a chain a test decides.
Amended below: an `if` the program wrote stays. Amended by
[0879](0879-an-if-and-a-closures-captures-read-as-written.md): a branch's copy
written in a `let … else`'s `else` stays, the guard's exit.

Case: A ([0262](0262-when-rust-and-js-disagree.md)): each rewrite changes
nothing a program can observe; the JS reads as a person writes it.

## Context

A std body is lowered from the optimized MIR the toolchain ships (ADR
0561), which is shaped unlike a crate's own: rustc's drop elaboration has
run, so a drop tests what it drops where rust-js's own dataflow would have
known the answer; a `bool` is made, negated and tested along each path to
a shared block; and a block two paths reach is copied into each. The
review of the first compiled bodies read three shapes the app's bodies
never show (`@rust-js/std`'s `iter.js`, and `Peekable`'s `count`, `nth`,
`last` and `next_if` compiled to look):

- a test its early return decided, `if (option != null) { return option; }
  .. if (option != null) { dropIItem?.(..) }`, and a drop's test inside
  its own, `if (peeked != null) { if ($someValue(peeked) == null) { .. if
  (peeked != null) { if ($someValue(peeked) != null) { .. } } } }`;
- a boolean temporary assigned on each path and read once each time,
  `tmp = peekable.peeked != null; tmp$2 = !tmp; if (!tmp$2) { break bb5; }`,
  which `declarations` can't make one `const` of, assigned twice, and
  `inlined` folds only a `const`;
- a tail written into two branches, `if (peeked != null) { .. return v; }
  return v;`.

Tidy had no pass for the first and third: the app's MIR never left them.
Where it had a pass, `inlined`, it was of a `const` alone.

## Decision

**Three rules of tidy's, general, with one owner each, no std-only case
(AGENTS.md's one way):**

- **A test an earlier one decided goes** (`branches::decided_tests`): in
  `if (t) { a } else { b }`, `t` holds through `a` and fails through `b`;
  where `a` leaves, `t` fails through what follows the `if`, and where `b`
  leaves, holds. An `if` of `t` there is its `then` or its `else`; one of
  `!t`, the other way. Only of a `t` that does nothing and reads no variable
  anything assigns, `changing`'s; one that reads through a field or a
  helper, `x.value`, `$someValue(x)`, is known only up to the next call or
  assignment through a field, which may change it. A drop flag whose only
  test went with such a drop is read nowhere now, and `flags::settled_flags`
  removes it with its writes, run again after.
- **A temporary assigned, then read once by the statement after it, each
  time, is read where it's made** (`bindings::paired_temporaries`, folded by
  `inlined` as a `const` read once is): `let _5; .. _5 = a; f(_5); .. _5 =
  b; g(_5);`, each assignment paired with its one read and nothing else
  reading `_5`. The `let` nothing names then goes, in a block as in the
  body.
- **A tail both branches end with is written once, after them**
  (`branches::shared_tails`): `if (c) { a; x = 1; } else { b; x = 1; }` is
  `if (c) { a } else { b } x = 1;`; and of an `if` without `else` whose
  branch ends as what follows the `if` does, `if (c) { a; return v; }
  return v;`, the branch's copy goes, `if (c) { a } return v;`. What does
  the same is `decisions::same_stmt`'s question, a match's arms' too: an
  assignment, an expression, a `throw`, a `return` or a jump written alike,
  not an `if`, so a drop's test stays beside the `try { .. } catch` that
  drops the same value on a panic, which `scoped_drops` makes its `finally`
  of. Three things stay where they are, each for a pass that owns the
  shape: a tail that leaves, `return v`, in two branches, whose exits are
  the branches' shape, the guard clause `let_else_first` and
  `early_returns` make, and a branch's copy of a `return;` of nothing, or
  of `undefined`, a `break` or a `continue`, the function's end
  `tail_returns` and a loop's exits `early_breaks` write as guard clauses;
  a branch that is only the tail, an arm `decisions` merges with the one
  alike, `s.TAG === "Rect" || s.TAG === "Line"` in written order; and a
  tail that reads what a branch declares.

Each runs where the app's passes run, twice: once the body's `const`s are
declared, and again once temporaries are read in place, before `and_tests`;
`shared_tails` once more after `returned_in_branches`.

## Why

- `std/iter.js`'s `next_back` has no test after its early return; the
  bodies compiled to look, `count`, `nth` and `last`, lose their drops'
  tests inside their own and `nth`'s second `return v`.
- The app's JS is unchanged or better, read case by case: a dead `else if
  (top != null)` after `if (top == null)` (`examples/strings.rs`), `else if
  (top) { $peekMutDrop(top) }` after `if (top)` (`heap_peek_mut`), a drop's
  test inside its own and an unread flag with it (`subscriptions`,
  `or_pattern_part_moves`), `if (tmp$2) {}` of nothing (`adapter_drops`,
  `loop_drops`, `extract_if_drops`), `let result; if (c) { result =
  undefined } else { result = undefined }` as `const result = undefined`
  (`question_drops`), and `tmp$7 = make(8); tmp$8 = [tmp$6, tmp$7]` as
  `tmp$7 = [tmp$6, make(8)]` (`temporaries_taken_apart`). Each is a JS
  rewrite that changes nothing that runs, the corpus against native Rust.
- One rule each, where the app's bodies get theirs: not a pass over std
  bodies alone, which would be a second way to tidy a body.

## Alternatives

- **Rust-js's own drop dataflow over optimized MIR**, to leave no dead
  drop test. Rejected: optimized MIR's switches are rustc's elaboration's
  own, which ADR 0561 reads as given; and the dead test is one case of a
  decided test, which a JS rule covers for the app's bodies too.
- **A tidy of std bodies apart from the app's.** Rejected: two ways to tidy
  a body (AGENTS.md).
- **Hoisting a leaving tail too**, `if (c) { a; return v } else { b;
  return v }` as `if (c) { a } else { b } return v`, and a branch's copy
  of a bare `return;` or a `break`. Rejected after the corpus: it turned
  guard clauses into nesting, the playground's `addFile`, a `let .. else`
  inside a branch and a `while let`'s exit, which `let_else_first`,
  `early_returns` and `early_breaks` write as the program did.

## Consequences

- `std/iter.js` is regenerated, `bun run std`, and the compiler rebuilt
  with its manifest.
- A test that reads through a field is decided only up to the next call:
  `peekable.peeked != null` after `peekable.peeked = undefined` with a drop
  between stays, though Rust's ownership says nothing else reaches it.
  Knowing that is MIR's lowering's, not tidy's: next.
- Nested tests of a std body, `if (peeked != null) { if ($someValue(peeked)
  != null) { .. } }`, aren't one `&&`: `and_tests` folds where the program
  wrote `&&`, which a std body, with no THIR, never says. Next.

## What holds it

- Unit tests of each rule in `tidy.rs`: `a_test_an_earlier_one_decided_goes`,
  `a_temporary_assigned_then_read_once_each_time_is_read_where_its_made`,
  `a_tail_both_branches_end_with_is_written_once`.
- `test/std-package.test.ts`: the committed `std/iter.js` is what the
  compiler writes, and has no test its early return decided, no once-read
  `bool` and no tail written twice.
- The corpus cases above, their JS snapshots reviewed; `examples/strings.rs`'s;
  and `test/corpus/decided_tests_past_a_call.rs`, against native Rust: a
  field's test after a call that clears the field is read again.
- Mutations `decided-tests-kept`, `decided-test-past-a-call`,
  `paired-temporaries-unpaired`, `shared-tails-kept`,
  `shared-leaving-tail-hoisted` and `unread-flag-kept-after-decided-tests`.

## Amended (2026-10-25): an `if` the program wrote stays

Case ([0262](0262-when-rust-and-js-disagree.md)): A, the same code run;
the test the program wrote is kept, which is true where it's tested.

React's `startGestureTransition` throws where `enableGestureTransition`
is off, then writes, under `// $FlowFixMe[constant-condition]`, `if
(enableGestureTransition) { currentTransition.gesture = provider; }`. Its
port's second test was decided by the first and folded to its branch,
`currentTransition.gesture = provider;`, and the comment written on the
`if` went with it.

**A test that an earlier one decides goes only where MIR made it, never
an `if` the program wrote** (`Written::ifs`, each `if`'s condition as the
program writes it, an `if let`'s too): that `if` is what the program
says, upstream's too. A std body's tests, a drop's of what an early
return gave away, a `match`'s arms tested one after another, and a part
of a chain (ADR 0743) still go. `option_bool_tested`'s `else if flag ==
Some(true)`, which its first test decides, is kept with them.

Held by `test/lowering-written-branches.test.ts`, "an if the program
wrote stays where an earlier test decides it", run; the mutations
`written-if-decided-away` (`mir/tidy/branches.ts`) and
`written-ifs-unseen` (`mir/hints.ts`).
