# 0364. Bodies are lowered from their MIR

Status: Accepted. MIR is the one lowering of bodies; THIR's is retired.
Amended: a type is an iterator where rustc says its `Item`, a type or
another trait's associated type, smallvec's `A::Item`, not one of std's
iterator traits' still: `implements_iterator`, which `for _ in &mut it`
asks (`test/corpus/iterator_of_projection.rs`). Amended by
[0569](0569-a-temporary-is-named-as-rust-analyzer-suggests.md): a
temporary tidy keeps is named as rust-analyzer suggests, after its call or
its type, before `value`. Extended by
[0751](0751-a-binding-only-a-guard-reads-isnt-written.md): a binding only
a guarded arm's guard reads isn't written in the arm. Amended by
[0800](0800-a-drop-of-nothing-never-unwinds.md): a drop of what JS drops
nothing of never unwinds, so the cleanup it would unwind to drops nothing.
Amended: a value a block gives last is given first only where nothing can
tell (issue #219, below); an `|` of two values' tests is a chain of its
arms; no arm holds a copy of what follows the `match` (kj-switch); a
chain's arm needs no more than that the arms before it failed (#221); a
tree of a `switch`'s guard is the `switch`'s (rd619); a test of a value an or-pattern's alternatives after it each say is theirs (#617); paths alike but for one value's test are one, and a path no alternative of whose or-pattern holds is none (#631); an alternative's tests of a tuple's parts are in the order written (#632); a match of three values is the chain as written, a test whose failing says another path of its arm, or that the arms before it say, is none (#673). Amended by [0879](0879-an-if-and-a-closures-captures-read-as-written.md): the value of an `if` the program wrote with a branch of `false` or `true` is a conditional, `c ? x : false`, not `c && x`; its branches are in the order written.

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

rust-js lowers each body from its THIR, the tree of what was written. JS
written from a tree reads as the Rust does, but what the tree leaves
implicit, rust-js had to work out case by case, each an ADR: the order
operands run in and when a value is read (a key against its children,
a struct's base after its fields, `unwrap_or_else`'s operand), how long a
temporary lives (a `RefCell`'s guard across JSX, ADR 0328), what a
reborrow or a coercion refers to, what's moved and what's dropped (ADR
0098), and on a panic, what's dropped as it unwinds.

MIR, what rustc builds for borrowck, says each of these: every value in a
temporary of its own, assigned in the order it's made, each borrow, move,
drop and check a statement or a terminator of its own. It's what Miri
runs, the semantics rustc defines.

## Decision

**Correctness is mandatory; readability is secondary.** MIR supplies
execution facts, but using MIR does not itself prove that rust-js's lowering
is correct. Tests and mutations must establish that the generated program
preserves the supported Rust semantics and documented differences.
Necessary temporaries, checks, helpers and cleanup blocks are acceptable;
readability rewrites are made only where they preserve semantics.

**A body is lowered from its MIR, as borrowck reads it, after promotion
and before drops are elaborated. Its THIR is beside it, for its shape:
names, spans, what a std macro made.** The JS is correct as it's made,
and made readable only where that's proved to change nothing:

- **Structure**, by Ramsey's "Beyond Relooper": a reducible graph, as
  Rust's are, is labeled blocks and loops, `break` and `continue`. A
  panic is a JS exception, so unwind edges and cleanup blocks are left
  out, and so are the edges borrowck alone reads.
- **Returns where they're written**: a branch to the block that only
  returns is the `return`; the return place assigned and returned is the
  value returned; a branch that returns or throws is written first, as an
  early exit, and what follows it unnested. A `break` that ends its own
  block, and a label nothing names, go.
- **`?` is what it tried**: `Try::branch` of an `Option` or a `Result`
  makes no `ControlFlow`; its `Continue` is the value inside, its `Break`
  the `None` or the `Err` itself, and `from_residual` THIR's conversion.
- **Folding**: a temporary made once and read once is written where it's
  read, where that changes no order. Those read that do something are
  read in the order they were made; what was made after them, and isn't
  read, moves freely; what's read before them can't change meanwhile.
  Otherwise each is a `const` where it's made.
- **What moves freely**: what does nothing and reads variables nothing
  borrows; and a place shared, or shown by `format_args!`, which borrowck
  keeps unchanged while the reference lives, read where it's used, as
  Rust reads it.
- **One std layer**: a std function's JS is made of its arguments' values
  and types, `std_values`. A handler that needs what THIR shows of an
  argument says so, and of a call from MIR takes the answer that's always
  correct: a `RefCell`'s borrow counted, an index checked.
- **Drops, as rustc elaborates them**: rustc's own dataflow of what's
  initialized says, before each `Drop`, whether what it drops is there,
  isn't, or may be, and a flag of its own, set where rustc's
  `ElaborateDrops` sets it, says which as the body runs. A call that may
  panic while something's there to drop runs in a `try` whose `catch`
  drops what its cleanup blocks drop and throws the panic on: the
  statement that runs it, as it may be written where its value is used,
  so the calls written in one statement share a cleanup, and a flag set
  or a different cleanup runs what's made first.
- **Representations are types'**: a cell of a local or a field (ADRs
  0270, 0288, 0362) stays a cell, made plain only where MIR's own analysis
  of its borrows says nothing else sees it change: a module's own
  thread-local, a cell only its function uses.
- **Integers wrap**: rust-js's session builds MIR with overflow checks
  off, as its semantics are the release profile's (ADR 0011).

## Why

- **Correct by construction**: each order, borrow and drop is what rustc
  says, not what rust-js worked out of a tree.
- **Readable by proof**: every rewrite toward the JS a person writes has
  a reason it changes nothing; where there's none, the JS is plainer, not
  wrong.

## How MIR became the one lowering

MIR became the default once correctness was shown for the supported scope:
the corpus agreed with native Rust under the documented semantics, the
supported rustc tests and mutations passed, react.dev's 823 pages kept
their behavior and public contracts, and diagnostics, interop and
native/browser consistency were checked. THIR's output was a readability
reference, not the authority on behavior.

Readability parity with THIR or the original JavaScript did not block the
switch. Output differences were reviewed for semantic and public-contract
changes; those that only affect readability are tracked as later
improvements, each of which must keep evaluation order, side effects,
copying, borrows, drops and failure behavior.

THIR's lowering of bodies is removed, and `RUST_JS_MIR` with it. Its last
uses each have a MIR lowering:

- **Writers** (ADR 0054): a `Formatter` parameter is the string `f` it
  writes to, a write `f += s`, and `f` what the writer gives back;
  `return s` of each way through that writes once. The `Formatter`'s
  options (ADRs 0058, 0137), its questions (`f.alternate()`, ADR 0143), a
  `Debug` builder's chain (ADR 0136), a writer that may fail (ADR 0187)
  and the `Formatter` given to generic `fmt::Write` code (ADR 0180) are
  lowered as THIR's were, and what THIR refused, a builder kept in a
  variable, is refused.
- **Initializers** (ADR 0096): a static's or a constant's lowered as code,
  a `const { .. }` block's, and a generic impl's constant (ADR 0176), from
  their MIR. Their checked arithmetic, which rustc evaluates, is the
  operation; a promoted value rustc can't say as JS, `&[("double",
  double)]`, is its own MIR lowered as code.
- **A trait's default copied into an impl** (ADR 0176): the default's MIR,
  as the trait has it, lowered in the impl's dictionary, its types asked in
  the impl's terms, an object `Self` given its impl's method as the object
  and a number's as its box.

A body whose MIR rustc evaluated before rust-js read it, a `const fn` a
pattern's constant calls, is read from the MIR rustc keeps for evaluation;
constants and `const fn`s are read first. What THIR refused only for its
own reasons and MIR lowers as Rust runs it is a corpus case that runs
against native Rust.

THIR is still read beside each body's MIR: for its shape, the names,
spans and what a std macro made, by which tidy writes the JS as it was
written, and for the facts its analyses give. Removing the lowering of
bodies didn't remove that.

## Amended by

- [0372](0372-a-check-that-drops.md): A check whose panic drops is written where it's made

## Amendment: what several cases test is made once

A `match` of a call's value, `match note(15, x) { 1 => .., 2 => .., _
=> .. }`, called it once per arm it tested: MIR folds the call into its
`switchInt`'s one use, and the switch is a test of each case. A
generated program found it, seed 50147 (2026-10-14). What more than one
case tests, and does something as it's made, is held first, `const n =
note(15, x);`, as Rust makes it once; one that only reads, `i % 3`, is
read by each test, as before. `test/corpus/match_call_once.rs` runs it
against native Rust; a mutation calls it again.

## Amendment: nested loops read as written

heck's word splitter, a `while let` with a `peek()` inside a `for`,
read as no one writes it: `let nextMode;` declared before the outer loop,
the inner loop in a labeled block, `bb103: { .. break bb103; return f; }
continue bb3;`, and a `return` after the `break` that nothing reached.
Each rule below changes no meaning:

- **A `let` sinks through a loop or a labeled block** that is all that
  names it, whose body assigns it before anything reads it: the variable
  Rust declares inside, `const nextMode = ..` where it's made.
- **A labeled block followed by a labeled jump**, `l: { .. break l; .. }
  continue m;`: each `break l` is that jump, which goes on from there
  anyway, and the label nothing names then goes.
- **A loop that ends the body of loop `l`**, one the program names no
  label: each `continue l` of its own, not a loop's inside it, is its
  `break`, as Rust's `break` there is, and `l`'s label goes; then an
  early exit, `if (!value) { break; }`, as before. A `continue 'rows` the
  program wrote stays. This runs once what's after each loop is placed,
  which reads where its `continue l` goes.
- **What those jumps leave past them goes**, as it did before them.

`test/corpus/nested_loop_jumps.rs` and heck's `transform`, in
`writer_callables.rs`, hold each, against native Rust; a mutation of each
is caught.

## Amendment: tidy's passes are one list

Tidy's rewrites, which a body's JS goes through in order, are one list of
stages, each a run of named passes and what it leaves a body with, checked
in a debug build; `--tidy-until` shows what each does
([ADR 0659](0659-tidy-is-a-list-of-stages-with-contracts.md)).

## Amendment: a default is given first only where nothing can tell

A block MIR made of a value, `bb7: { if (c) { x = 1; break bb7; } x = 0;
}`, `matches!`'s, gives its last value first, `x = 0;` before it, as a
person writes a default. Porting React's `mapIntoArray`, it gave
`invokeCallback = true` before testing `$$typeof`, so every object called
back (issue #219): the rule counted the assignments just before a `break`
of the block, but not the `break`s with none, an arm `_ => {}`'s, which
went on past the block holding the default. It reads now as what can see
the value:

- **Each other way out of the block gives its own**, just before a
  `break` of it. A `break` of it after anything else keeps the block.
- **A variable declared in the block's list**, `let v;`, which nothing
  names after it but the block: nothing sees it before the block ends, and
  a way out of the block another way, a `return`, a `throw`, a jump to
  past it, leaves its scope. Given first, as before: `let v = if let ..
  { q } else { 0 }`'s `let v = 0;`.
- **Any other**, a variable from before, a closure's captured one, a
  loop's: what the block runs before its end neither runs code nor throws
  (`Expr::may_throw`, `Stmt::only_makes_values`), and leaves only by a
  `break` of the block, of its own loop or `switch`, or its end. A call
  could read it through a closure; a `continue`, a `break` of a loop
  around it, a `return` from a closure, or a panic `catch_unwind` takes
  would see it given.

Case: results change ([0262](0262-when-rust-and-js-disagree.md)), Rust's
kept. `test/corpus/default_value_exits.rs` runs each against native Rust:
the arm that does nothing, a loop's `break` and `continue`, a closure's
`return`, a panic in `catch_unwind`, a call and a guard's call reading the
variable through a closure; and a `let`'s value still given first.
React's shape is upstream's `switch (children.$$typeof) { case
REACT_ELEMENT_TYPE: case REACT_PORTAL_TYPE: invokeCallback = true; break;
case REACT_LAZY_TYPE: .. }` (`test/lowering-switches.test.ts`). A mutation
of each condition is caught, by a match of symbols too, whose tree tidy
can't chain (`test/lowering-switches.test.ts`, each against what native
Rust gives of the same program of `Option<u32>`s).

### An empty array is given first as a constant is (#556, 2026-11-04)

The playground's `prepare`, `let entries = match (test, tests) { (true,
Some((module, files))) => .., (false, _) if main => vec![root], _ =>
Vec::new() };`, printed a labeled block, `bb17: { if (test) { if (tests
!= null) { ..; break bb17; } } else if (main) { ..; break bb17; }
entries = []; }`: the `_` arm, which two failures share, is the block's
end, and only a constant was given first. **An empty array, `Vec::new()`'s
`[]`, is given first as a constant is**: it runs no code, and nothing
holds it but the variable, so a way that gives its own leaves it unseen,
and it's new each time the block runs. So `let entries = []; if (test) {
if (tests != null) { .. } } else if (main) { .. }`, as `let n = 0;` is
of a number. The playground's `prepare` is its `match` again (446ad7253
had rewritten it as an `if`/`else` chain around this).

Case ([0262](0262-when-rust-and-js-disagree.md)): N. Held by corpus
`let_match_empty_default.rs` against native Rust (each array pushed to
after, so a shared one would show) and the mutation
`empty-array-not-given-first` (`decisions/defaults.ts`).

## Amendment: an `|` of two values' tests is a chain of its arms

That case's `(1, _) | (_, 1)` kept the labeled blocks MIR made of it, `bb15:
{ if (a !== 1) { if (b === 1) { break bb15; } .. } } hit = true;`, which
the wrong default had hidden. The chain of a match's arms (`decisions`)
is made of it now, `if (a === 1 || b === 1) { .. } else if (a === 2 && b
=== 2) { .. }`: the chain was longer than the tree, which it stayed, and
two rules make it no longer, each changing no meaning:

- **A path's test of a value a later one says more of**, `a !== 1` before
  `a === 2`, is that one, in its place: each test is still read only where
  it was.
- **An alternative of one test**, `a === 1`, is tried before those ahead
  of it that each test its value first, where one of them says it fails;
  and what its failing says is left out of each after it: `a === 1 || b
  === 1`, not `a === 1 || (a !== 1 && b === 1)`.

Case: none (ADR 0262): the tests are pure, so the chain gives what the
tree did. `test/corpus/default_value_exits.rs` runs it against native
Rust, `NaN` and three values' `|` too; of a slice's patterns, which these
don't make a chain of, the blocks stay (issue #221). A mutation of each
rule is caught.

## Amendment: a value held out of a list is in its `try`

Case: C ([0262](0262-when-rust-and-js-disagree.md)): the answer was
wrong, a destructor not run as a panic unwound; it's Rust's now.

A statement that runs a call made before, which may panic while something
is there to drop, is the call's `try`. The statement path asked whether it
ran one by whether a value that acts was no longer made and unread. A
`format_args!` argument read out of the order it's made in, a named one,
`println!("{total} {}", first(&v))`, holds each argument that does
something first, `const _arg = first(v)` (`take_field`), and the list
they're in stays made, its other items unread: the statement ran the call,
and wasn't its `try`. A panic in `first` dropped nothing that was live, a
guard's `drop outer` not printed. Found reading why an architecture
review's example didn't make one `finally` of its guard's scope (ADR
0437).

- **What a statement ran is counted by what acts, a list's items each
  one** (`acting_parts`), as `take_field` holds them: not movable, and
  doing something. Fewer after it, it ran some, and is the `try`.
- **The scope is one `finally` again**, as ADR 0437 makes of it, now that
  each statement that may throw in it is wrapped; `refcell_eq_through_guard`'s
  scope is too. `generic_drops_nothing` and `rc_counted_uses` hold a value
  made of a call in a `try` of its own, each of which a panic in its call
  dropped nothing before, the same bug.
- **A held argument a `try` assigns is `arg`**, as one held in a `const`
  is, `let arg; try { arg = .. }`, not MIR's lowering's `_arg`.
- Held by `test/corpus/format_argument_unwinds.rs`, against native Rust,
  and the mutations `held-format-argument-unwound-bare` and
  `held-arg-let-unnamed`.

## Amendment: no arm holds a copy of what follows the `match` (kj-switch, 2026-10-29)

```rust
for &x in xs {
    match x {
        0 => { out.push(1); continue; }
        1 => out.push(2),
        _ => {}
    }
    if x > 5 { out.push(x); continue; }
    out.push(-x);
}
```

MIR's block of the `match`, `bb20: { if (x === 0) { ..; break bb20; } if
(x === 1) { .. } if (x > 5) { .. } else { .. } }`, ends in the `if` the
program wrote after the `match`, and `decisions` walked into it as one of
the tree's tests: the arm `1` went on to the rest of the block, the whole
`if`, and the leaves of the `_` were its branches, so the chain held it
twice, `else if (x === 1) { out.push(2); if (x > 5) { .. } else { .. } }
else if (x > 5) { .. } else { .. }`, a `switch` of it then each case's
copy, and arm `0`'s `continue` the `switch`'s `break;`.

**A tree one of whose arms holds a statement another holds a part of is
no chain of arms**, as one that copies more than a jump isn't: the block
stays for the later passes, whose `continue` it's followed by makes each
of its `break`s the program's `continue`, and `switches` reads the arms
before the `if`, `switch (x) { case 0: out.push(1); continue; case 1:
out.push(2); } if (x > 5) { out.push(x); continue; } out.push(-x);`, as
written. No other corpus case or port module changed.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. The same tests in
the same order, each statement run as often.

Held by `test/corpus/match_arm_continues.rs` against native Rust.

The mutation `decisions-tail-copied`, the refusal gone, survived, as
`decisions` refuses a chain whose tests are longer than the tree's: the chain that copies the `if` into the `else` reads
its tests again, 8 and 7 atoms where the tree's are 5 in the case's
`numbers` and `bytes`, so `tests_made > written` refuses it too, and no
tidy pass's output differs. The whole corpus run without the refusal
changes nothing, so the mutant is equivalent and removed (2026-11-04).
The refusal stays: it's the rule's reason, said where it is, which the
count only meets.

## Amendment: a chain's arm needs no more than that the arms before it failed (#221, 2026-10-29)

A slice's `match`, `[1, ..] | [.., 1] => hit = true, [2, 2] => return, _
=> hit = false`, is a tree MIR tests in parts: `row.length >= 1`, then
`row[0] === 1`, then the last, then `row.length === 2` and each `2`.
`decisions` gave each arm every test on the way to it, `[2, 2]`'s
`row.length >= 1 && row[0] !== 1 && row[row.length - 1] !== 1 &&
row.length === 2 && ..`, and the `|`'s two paths each their own `row.length
>= 1`, more tests than the tree has, so it kept MIR's block, `bb11: { ..
break bb11; .. }`.

**In the chain, an arm needs no test that says only that an arm before
it failed**: a path through an earlier arm's path's tests but its last,
then that last failing, needs no more of that last, as the chain got past
that arm, each earlier path in turn while one says more; what's left is
read as before, `row.length >= 1` and `row.length === 2` one test. **And
what each of an arm's paths tests first alike is tested once**, `n >= 1 &&
(a === 1 || b === 1)`; the tests are pure, so neither changes what's
read or found. Not of an earlier arm of one test, `a === 1 | ..`, whose
failing the tree's order of tests says already as written, `a === 2 && b
=== 2`. React's slice matches read as written: `if (row.length >= 1 &&
(row[0] === 1 || row[row.length - 1] === 1)) { .. } else if (row.length
=== 2 && row[0] === 2 && row[1] === 2) { .. } else { .. }`.

A guard that calls, `[n, ..] if below(*n)`, is no test the tree can move
and still keeps the block (`guarded_rows`, `odd_ones`); so does a `match`
of symbols tested inside an `if` of their `Some`, the rest of #221.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. The same pure tests,
the same arm taken.

Held by `test/corpus/default_value_exits.rs` against native Rust;
mutations `decisions-failed-arm-kept` and `decisions-shared-tests-repeated`
(`mir/tidy/decisions.ts`).

## Amendment: a guard that calls is a test of the chain (#221, 2026-10-29)

Case ([0262](0262-when-rust-and-js-disagree.md)): N. The same calls, in
the same order, the same arm taken.

`decisions` read a tree of pure tests only, whose arms it may try in any
order. A guard that calls, `[n, ..] if below(*n) => level = 3`, was what
an arm did, so the tree kept MIR's block, `bb25: { if (row.length >= 1) {
.. const n = row[0]; if (below(n)) { level = 3; break bb25; } } level = 5;
}` (`guarded_rows`), and `(1, x) | (x, 1) if odd(x)` its `bb30:`
(`odd_ones`).

**A test of a guard the program wrote is a test of the tree too**
(`Written::guards`), with the binding MIR gives it just before, `const n
= row[0];` or `x = b;`, in its place, `below(row[0])`, where only that
test reads it (`bound`, `binds`): a value of the scrutinee, which nothing
changes before the call reads it. A guard's own bindings are its, `n$1`
beside its arm's `n`, so the arm reads what it reads as before. Not of a
`match` that's a `switch` (ADR 0629), whose cases test their guards.

**A tree that calls is tried each way its tests come out**
(`decisions/calls.rs`, `Model`): each value it tests, apart from the
others, over the values its tests tell apart, and each call true or
false, at most 4,096 ways. Of each, the tree reaches one arm, making its
calls in order; the chain must too. Each arm's test, after the arms
before it, starts from the paths the tree takes to it, less each test
what's tried before says (`x !== 1` once `x === 1` failed, a call's
`!odd(b)` once `odd(b)` was made and failed), then one at a time each test
that can go with the paths it then says all of, each step tried each way:
the arm passes where the tree reaches it with the calls the tree makes,
and fails otherwise having made only the calls the tree makes first; and
the whole chain does what the tree does. A test read after a call reads
what rustc keeps a guard from changing, a value its `match` tests of its
scrutinee (E0510, `Guard::tested`), or the tree stays: `[n, ..] if mark(*n)
&& done.get()`, whose call sets `done`. Two calls written alike, the same
guard of each `|` alternative of no binding, `(1, _) | (_, 1) if odd(a +
b)`, the model can't tell apart, and the tree stays. Its tests are counted
once each alike, `row.length >= 1` of `[1, ..]`'s arm and of `[n, ..]`'s,
against the tree's as read.

`guarded_rows` is `row.length >= 1 && (row[0] === 1 || row[row.length -
1] === 1) ? 1 : row.length === 2 && row[0] === 2 && row[1] === 2 ? 2 :
row.length >= 1 && below(row[0]) ? 3 : 5`, and `odd_ones` `(a === 1 &&
odd(b)) || (b === 1 && odd(a)) ? 1 : ..`; test/corpus/default_value_exits.js
has no label.

Alternatives: a guard's test kept in place, the rest of the tree a chain
around it, which reads as MIR's tree does; and the rules the pure tree's
chain is made by, which say nothing of when a call is made. A model of
each way is what tells, whatever the tree's shape, and refuses where it
can't.

Held by `test/corpus/guards_that_call.rs` and
`test/corpus/default_value_exits.rs` against native Rust, each guard's
calls printed; mutations `decisions-guard-call-a-leaf`,
`decisions-guard-binding-not-given`, `decisions-switch-guard-chained` and
`decisions-call-tree-tests-repeated` (`mir/tidy/decisions.ts`), and
`calls-read-after-unkept`, `calls-nothing-said` and `calls-no-fewer`
(`mir/tidy/decisions/calls.ts`). What reads a binding but its guard, the
alike calls and the last check of the whole chain have none: MIR makes no
such tree the corpus has found, and each arm's own check says what the
last does of each tree it has.

## Amendment: a constant's test says its value is no `None` (#412, 2026-10-29)

A `match` of an `Option`'s constants whose arm leaves the loop, `Some(C)
=> break`, is no `switch` (its `break` would need a label the Rust lacks)
and so the `if` chain `decisions` makes; but each arm's path there began
`symbolFor != null`, so the chain had more tests than MIR's tree, and
the tree stayed, `bb22: { if (symbolFor != null) { .. break bb22; .. } hit
= false; }`.

- **A path's `x != null` where another of its tests is `x === k` of a
  `k` that's never `null` or `undefined` goes** (`present_said`): a
  string, a number, or a constant of a `match` whose constants are
  numbers, strings or the registry's symbols (`Switch::valued`), as ADR
  0629's `switch` takes `None` in no case. `x === k` is false of `null`,
  so the path holds alike.
- **Of an arm's alternatives MIR tests apart, each failing of one before
  it that's a single test no set says, a symbol's, goes from the paths
  after it** (`alternatives`): `s === A || (s !== A && s === B)` is `s ===
  A || s === B`, as `a || (!a && b)` is `a || b`; the tests are pure.

- **Only of a tree no `switch` takes**, where an arm or what follows the
  block breaks the loop around it (`Told::loops`), which a case would
  take for the `switch`'s: of the others, a `switch` of the value reads
  as written, React's resolveThenable's `throw thenable;` after it, not
  its `default`, and a `match` with a `None` arm stays as it was.

`if (symbolFor === A || symbolFor === B) { hit = true; } else if
(symbolFor === C) { break; } else { hit = false; }`, and `stopped`'s
`continue` and `break` alike. Case ([0262](0262-when-rust-and-js-disagree.md)):
N, each value takes the same arm. Held by
`test/lowering-switch-symbols.test.ts` (`broken`, `stopped`, run in Node
with native Rust's values), corpus case `option_match_leaves_loop` against
native Rust, and mutations `present-unsaid`,
`present-unsaid-past-a-failure`, `negated-alternative-kept`,
`present-said-of-any-tree`, `present-said-past-the-block-unseen`
(`mir/tidy/decisions.ts`). No mutation of `Switch::valued`'s part: each
constant MIR makes a `===` of is one.

## Amendment: an arm that does nothing between others needn't be the `else` (bb39, 2026-10-30)

```rust
match name {
    "B" if keep => continue,
    "C" => {}
    "D" if keep => out.push("kept d".to_string()),
    _ => out.push(format!("other {name}")),
}
out.push(format!("tail {name}"));
```

kept MIR's block, `bb39: { if (name === "B") { if (keep) { continue; } }
else if (name === "C") { break bb39; } else if (name === "D" && keep) {
..; break bb39; } out.push(..); }`, a label the Rust lacks. `decisions`
made the arm that does nothing, `"C"`, the chain's `else`, so the `_`
arm needed a test of each path to it, `(name === "B" && !keep) || ..`,
more tests than the tree's, and the block stayed.

Now, where that chain has more tests than the tree, and the block goes on
to something, `decisions` tries the arm whose test is longest as the
`else` (`arranged`), as of any other tree, the arm that does nothing an
`if` of an empty `then` in its place, which reads as its test negated
around the arms after it, as a `match` that ends its function already
reads:

```js
if (name === "B" && keep) {
  continue;
}
if (name !== "C") {
  if (name === "D" && keep) {
    out.push("kept d");
  } else {
    out.push(`other ${name}`);
  }
}
out.push(`tail ${name}`);
```

Not where the block ends a loop's turn, `"SCRIPT" | "STYLE" => continue,
"LINK" if keep => continue, _ => {}` last: that tree is upstream's
`switch` of `continue`s (rd184, ADR 0636), which a negated chain would
take. A `match` of symbols in an `if` of their `Some` whose `_` breaks
the loop, and whose arms go on to what follows (`levels` of
`test/lowering-switch-symbols.test.ts`'s runtime test), still keeps its
block: its inner block's chain has more tests than its tree, as symbols
are no values `simplified` reads apart (`s !== A && s === C`), and the
outer block's tree can't hold the inner block.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. The same pure tests,
each value taking the same arm.

Held by `test/lowering-labels.test.ts` ("an arm that does nothing between
guarded arms is no labeled block", run in Node),
`test/corpus/guarded_arm_falls_to_tail.rs`'s `kinds` against native Rust,
and the mutations `empty-arm-else-only` and `empty-arm-of-a-turn-negated`
(`mir/tidy/decisions.ts`). `default_values` and its mutations moved to
`decisions/defaults.rs` (`decisions/defaults.ts`), the file at its cap.

## Amendment: a presence test each of a chain's tests says is the chain's (bb11, 2026-10-31)

A `match` of `Some`s of constants whose other arm leaves its loop, its
value used after, `match tag(value) { Some(A | B) => level = 1, Some(C)
=> level = 2, _ => { seen.push(0); break; } } seen.push(level);`, kept
MIR's block: `bb11: { if (symbolFor != null) { if (symbolFor === A ||
symbolFor === B) { level = 1; } else if (symbolFor === C) { level = 2; }
else { break bb11; } seen.push(level); continue; } } seen.push(0);
break;`. The decision tree of the whole block makes more tests than the
tree (the arms' shared `seen.push(level)` is no jump to copy), and a
`switch` can't take the loop's `break` in a `default`.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. Each `===` of a value
fails where the subject isn't there, so a `null` reaches the chain's
`else` as it reached what follows the block; a test of the chain that
doesn't say so is the presence test's too, as it was inside it.

- **`l: { if (x != null) { lets; chain; rest } } e`, the chain's last
  `else` `break l;` alone and no other `break l`, `rest` and `e` each
  leaving: `lets; chain; rest`, the chain's `else` `e`**
  (`decisions/presence.rs`, pass `presence said` before `switches`). A
  test that says `x` is there, `x === v` of a value (`decisions::valued`,
  now shared with the tree's `present_said`, and `says`), stays; one that
  doesn't is `x != null && t`: `if (value === 1) { .. } else if (value !=
  null && value !== 3 && value !== 9) { .. } else { seen.push(0); break; }`.
  The block's `let`s declared first go before the chain, where no `let`
  of their names is before it.

Held by `test/corpus/presence_said_by_chain.rs` against native Rust,
`test/lowering-switch-symbols.test.ts`' "a match of symbols whose other
arm breaks its loop leaves the loop" (no label), and mutations
`presence-said-never`, `presence-said-of-every-test`
(`tidy/decisions/presence.ts`). The shape's other requirements (`rest` and
`e` leaving, no other `break l`, no `let` of the name before) no known
MIR shape reaches; they keep the transform exact where one would.

**Amended: a guard whose failing breaks the block.** `match value {
Some(1.0) => level = 10.0, Some(x) if x.is_nan() => level = 0.5, _ => {
seen.push(0.0); break; } }` kept the block, its guard's link MIR's way
round: `bb10: { if (value != null) { let level; if (value === 1) { level
= 10; } else if (!Number.isNaN(value)) { break bb10; } else { level =
0.5; } seen.push(level); continue; } } seen.push(0); break;`. A link
whose branch is the block's `break` alone is turned first, its test the
guard's and that `break` its `else`, which is the chain's last then, so
the rule above takes it: `let level; if (value === 1) { level = 10; }
else if (value != null && Number.isNaN(value)) { level = 0.5; } else {
seen.push(0); break; } seen.push(level);`, the test as written and no
label. Turning an `if`'s test and swapping its branches changes nothing
it runs (case N). Held by the same case's `halves` (NaN and other
numbers, `None`) and mutations `presence-guard-link-unturned` and
`presence-guard-link-turned-untested`.

## Amendment: all of rust-js runs with rustc's stack

Case ([0262](0262-when-rust-and-js-disagree.md)): none; where the work
runs, not what it makes.

rustc's thread was given a 256 MB stack for lowering, as deep as a function
is long, but rust-js wrote the JS after that thread ended, on the process's
main thread: 8 MB. rustc's `mir/issue-29227.rs`, a `match` of hundreds of
arms, is an `else if` chain as deep, and printing it overflowed that stack
on macOS, aborting, where Linux's frames fit. **rust-js runs on a thread of
its own with the stack rustc's thread has**, `RUST_MIN_STACK`'s if it's
set; a panic there ends the process with 101, as one on the main thread
did. The rustc suite's `issue-29227.rs` run on a Mac is what holds it.

## Amendment: tests each path ends with alike are tested once, a guard's call too (rd136, 2026-11-02)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the same tests in
the same order.

MIR tests a guarded or-pattern's guard of each alternative, so an arm's
paths end alike: `(a === 1 && on) || (a === 2 && on)`. **What each path
tests last alike is tested once, after**, `(a === 1 || a === 2) && on`,
as `1 | 2 if on` is written (`factored`); of pure tests it's the same
truth. A guard that calls was refused a chain where the tree has two calls
alike, MIR's copies of one guard, as the model read them as one: **calls
alike are one call of the model**, which refuses only a path that makes
two, `(1, _) | (_, 1) if odd(a + b)` of `(1, 1)`, which Rust calls twice.
`Some(1) | None if odd(3)` is `(a == null || a === 1) && odd(3)`, where it
was MIR's labeled block. See ADR 0905's amendment of the same day.

## Amendment: a tree of a `switch`'s guard is the `switch`'s (rd619, 2026-11-02)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, which statements are
made, not what they do.

React DOM's SelectEventPlugin moves `target_inst` in one arm, `"focusin"
=> { if .. { ACTIVE_ELEMENT_INST.set(target_inst); } }`, and has a guarded
arm that falls to the next, `"selectionchange" if skip => {}`. MIR decides
the drop through a block every arm breaks out of, and `decisions` read
that tree: the arm's own `if` as one of its tests, `name === "focusin" &&
(..)`, and the guarded arm with the next, `(name === "selectionchange" &&
!skip) || name === "keydown"`. Neither is a `switch`'s, and upstream's
`switch (domEventName)` was an `if` chain, the comment above the guarded
arm on its test's operand. Where the arms return, no block is made and
the `switch` takes the arms as written.

**A tree whose tests include a guard of a `match` that's a `switch`
(`Written::switches`) isn't made a chain** (`chain`'s `switched_guard`):
the `switch` takes it, its guarded case falling through to the next (ADR
0905), and each arm's `if` its case's. This replaces the rule that a
`switch`'s guard that calls is no test of the tree (#221), which left its
`if` to an arm's statements: one rule, as a guard of either kind is now
one the tree never chains. An `if` the program wrote in an arm's body was
considered for a rule of its own, that it's the arm's and no test, but no
tree reaches `decisions` with one where the `switch` has no guard: MIR
makes no block of it. And bb39's exception, that an arm doing nothing
where a loop's turn ends isn't the chain's negated test, as a `switch` of
`continue`s may be (rd184), was of a guarded `switch`'s tree, which never
gets there now: it's gone, and its mutation, which the corpus and the
lowering's tests all passed with, with it; `switch_turn_end_unguarded`
holds the `switch` of such arms with no guard.

Held by the corpus cases `switch_arm_tests_its_own` (the port's shape, its
moved `Rc`, and one with no guard), `switch_turn_end_unguarded`,
`guarded_arm_falls_to_tail`, `default_value_exits`,
`guards_that_call` and `guarded_arm_chains`, run against native Rust, and
the mutation `switched-guard-decided` (`mir/tidy/decisions.ts`), which
takes `decisions-switch-guard-chained`'s place.

## Amendment: a variable's default, given in each branch that fails to it (#567, 2026-11-04)

#556 gave an empty array first, as a constant is. A `_ => fallback` of a
variable, which `default_values` doesn't give first, still printed MIR's
labeled block, `bb10: { if (test) { if (tests != null) { ..; n = t + 1;
break bb10; } } else if (main) { n = 7; break bb10; } n = fallback; }`.
A JS label the Rust lacks is never accepted. **Where the block's last
statement assigns a variable, which reading again runs no code, and each `break` of the block ends a branch the block ends with,
the assignment is given at the end of each branch that goes on to the
block's end, `} else { n = fallback; }`, and the block is gone**
(`defaults::copied_into_branches`, the coordinator's decision on the
user's behalf). The same assignment runs at the same point of each path
as MIR's does at the block's end, so it's what MIR's does whatever the
value; the value is limited to what runs no code only so nothing is
written twice. Not a constant `default_values` can't give first: an
`Option` of symbols' `None` and `_ => hit = false`, which `switches`
makes the `switch`'s `default:` (`lowering-switch-symbols`). A computed
`_`, a call, keeps the labeled block, its form
left for the user to decide (#567).

Case ([0262](0262-when-rust-and-js-disagree.md)): N. Held by the corpus
`let_match_variable_default.rs` against native Rust, the mutation
`default-copied-into-no-branch`, and the unit test
`a_default_is_copied_only_where_each_break_ends_a_branch` (a `break` of
the block before its last `if`, which keeps the block).

## Amendment: a computed default is the `else` of the chain, its tests read again (#583, 2026-11-04, the user's decision)

#567 left a computed `_`, a call, to the user. `let entries = match (test,
tests) { (true, Some(t)) if cond => vec![t], (false, _) if main =>
vec![9], _ => compute(3) };` printed MIR's tree, `bb17: { if (test) { if
(tests != null && cond) { entries = [t]; break bb17; } } else if (main) {
entries = [9]; break bb17; } entries = compute(3); }`: `decisions` kept
the tree because the chain's tests, `test && tests != null && cond` then
`!test && main`, are longer than the tree's, which reads `test` once, and
neither `default_values` nor `copied_into_branches` gives a call without
the block. The user decided: **such a `match` prints as the chain of its
arms, `if (test && tests != null && cond) { .. } else if (!test && main) {
entries = [9]; } else { entries = compute(3); }`, no label.**

`decisions` takes the chain though its tests are longer where the tree's
block would keep its label (`keeps_label`): more than one path goes on to
the chain's `else`, which does what `defaults` doesn't give without the
block (`defaults::given_without_block`, the one answer of what it gives:
a constant or an empty array first, a variable in each branch), every
arm does something, each leaves the block or returns or throws, and no
`switch` takes the tree (no test in a `match` of constants, whose cases
need no label). A test read again is read only where it was, before any
arm runs, and only of tests whose reading does nothing (`pure` and no
`has_effects`, so no getter, `Known::Getter`); a tree whose guard calls
keeps `Model`'s rule. The tests MIR reads are of the scrutinee made once
before the tree, `const value = flag(test, log);`, so its call runs once.

The chain's last two arms, values, were then folded by `branch_values`,
`else { entries = !test && main ? [9] : compute(3); }`, half a chain, as a
written `if let .. else if .. else` printed already: the first arm does
more than give the value (`const t = tests;`), and `chained_arms` took
back only a temporary's chain of such a first arm. ADR 0969's rule holds
of a variable's now too, but the function's value, whose conditional may
be an `Option`'s `and_then`'s, no arm the program wrote
(`interpreter.rs`'s `Env.get`).

Case ([0262](0262-when-rust-and-js-disagree.md)): N, no observable
difference: the same tests of what nothing changes, the same arm taken,
the default run only where no arm is. Held by the corpus
`let_match_computed_default.rs` against native Rust (a `let`'s, an
`Option`'s and a statement `match`'s, the scrutinee's call logged), and
`match_of_what_a_match_made.rs`'s `instanceOf`, now the chain it's written
as; the mutations `computed-default-kept-labeled`,
`computed-default-of-an-empty-arm`, `computed-default-given-first`
(`lower/mir/tidy/decisions.ts`) and `first-arm-doing-more-half-folded`,
`function-value-chained` (`lower/mir/tidy/branches.ts`).

## Amendment: a test an or-pattern's alternatives each say (#617, 2026-11-04)

`(1, _) | (_, 1) => hit = true, (2, 3) | (3, 2) => { .. }, _ => hit =
false`'s tree tests `a === 1`, then `b === 1`, then the later or-pattern's
alternatives, `(b === 3 && a === 2) || (b === 2 && a === 3)`, one test of
the tree. The later arm's path, `a !== 1 && b !== 1 && (..)`, kept `a !==
1`: `simplified` reads a test of a value a later test of it says more
than as that one, `a !== 1 && a === 2` as `a === 2`, but no later test
here is one of `a`. The chain had a test more than the tree, so MIR's tree
stayed, the arms after the first first:

```js
if (a !== 1 && b !== 1) {
  if ((b === 3 && a === 2) || (b === 2 && a === 3)) {
    seen.push(true);
    continue;
  }
  hit = false;
  continue;
}
hit = true;
```

**A test of a value is said by a later `||` each of whose alternatives
says more of it** (`says_within`): `a === 2` and `a === 3` are each
within `a !== 1`, so the `||` takes its place, as a later test of it
would, each read where it was; and so does `b !== 1`, and the chain is
the tree's six tests:

```js
if (a === 1 || b === 1) {
  hit = true;
} else if ((b === 3 && a === 2) || (b === 2 && a === 3)) {
  seen.push(true);
} else {
  hit = false;
}
```

Not by leaving out every earlier arm's one test's failing, which #221
keeps (`a === 1 | ..`): that made `b'a' => break`'s `switch` a `while`
test and nested a `let`'s arms otherwise (`match_breaks_its_loop`,
`let_else_returns_in_arms`).

Case ([0262](0262-when-rust-and-js-disagree.md)): N. `a !== 1` holds
wherever the `||` does; each test is pure, so the same arm is taken.
Held by `test/corpus/tuple_or_pattern_arm_comments.rs` against native
Rust, every pair of 0..5, and `test/lowering-comments-inside.test.ts`'s
"a comment above a later arm of a tuple's or-pattern is the arm's", run;
the mutations `or-pattern-alternatives-say-nothing` and
`or-pattern-alternative-tests-unread` (`lower/mir/tidy/decisions.ts`).

Left: a first arm before them that tests both, `(0, 0) => .., (1, _) |
(_, 1) => .., (2, 3) | (3, 2) => ..`, whose paths to the `|` arm, `a ===
0 && b === 1`, `a === 1`, `a !== 0 && a !== 1 && b === 1`, the chain
can't read as `a === 1 || b === 1`, keeps MIR's labeled block (until
#631, below). Each
alternative's tests are in the order MIR makes them, `b === 3 && a ===
2`, not `a === 2 && b === 3` as written, where an earlier arm's
alternative tests `b` first (until #632, below).

## Amendment: paths alike but for one value's test are one (#631, 2026-11-04)

`(0, 0) => .., (1, _) | (_, 1) => .., (2, 3) | (3, 2) => .., _ => ..`
kept MIR's labeled block, its chain 18 tests to the tree's 8:

```js
let value = 2;
bb14: {
  if (a === 0 && b === 0) {
    log.push(0);
    value = 1;
    break bb14;
  }
  if (a === 1) {
    break bb14;
  }
  if (b !== 1) {
    if ((b === 3 && a === 2) || (b === 2 && a === 3)) {
```

The tree reaches the `|` arm three ways, `a === 0 && b === 1`, `a === 1`
and `a !== 0 && a !== 1 && b === 1`; and the later or-pattern's arm two,
`a === 0 && X` and `X && b !== 1`, X its alternatives' `(b === 3 && a ===
2) || (b === 2 && a === 3)`, which the tree tests past `a === 0` too. Three
rules of an arm's paths, each the same pure tests' truth
(`decisions/paths.rs`):

1. **Paths alike but for what each tests of one value are one**, that
   value's test of either's values, in its place (`unions`): `a === 0 &&
   b === 1` and `a !== 0 && a !== 1 && b === 1` are `a !== 1 && b ===
   1`, as `(x && s && y) || (x && t && y)` is `x && (s || t) && y`; then
   `a === 1`'s failing says its `a !== 1` (`alternatives`), `a === 1 ||
   b === 1`. Each test is still read where it was, and none is read that
   wasn't. Only of paths that test more than that value: a value's tests
   alone are its alternatives as written, `c === "." || (c >= "0" && c <=
   "9")` read in value order otherwise.
2. **A path whose or-pattern none of the alternatives of can hold, of
   what the path says of a value, is none** (`excludes`, in
   `simplified`): `a === 0 && X`, each alternative testing `a` of 2 or 3.
   Only where each alternative is false: `a === 2 && X` stays, the first
   holding.
3. **A test the alternatives before it each say goes**, `X && b !== 1`'s
   `b !== 1`, as one after it that says it takes its place (#617).

```js
let value;
if (a === 0 && b === 0) {
  log.push(0);
  value = 1;
} else if (a === 1 || b === 1) {
  value = 2;
} else if ((b === 3 && a === 2) || (b === 2 && a === 3)) {
```

The tests are what the tree reads, each a pure read of the scrutinee's
parts, made once before the tree, `const value = logged(log, a);`, so
none of the rules moves a call or a read that can throw: a value's test
stays where its path read it, and a path that goes is one no input takes.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. Held by
`test/corpus/or_pattern_paths_absorbed.rs` against native Rust, each of
a tuple's, a call's tuple's, a slice's, an `Option`'s and an enum's
`match` of every input of 0..5, and `test/lowering-matches.test.ts`'s "a
tuple's or-pattern after an arm that tests both values is an else if",
run; the mutations `paths-alike-not-one` (`lower/mir/tidy/decisions.ts`),
`paths-alike-but-for-two-values-one`, `or-pattern-excluding-path-kept`,
`or-pattern-excluding-one-alternative` and `or-pattern-said-atom-kept`
(`lower/mir/tidy/decisions/paths.ts`), the second and fourth wrong
answers native Rust catches. `simplified` and the rules of paths moved to
`decisions/paths.rs` with them, `decisions.rs` past the file cap.

Left: a match of three values, `(0, 0, 0) => .., (1, _, _) | (_, 1, _) |
(_, _, 1) => .., (2, 2, _) | (_, 2, 2) => ..`, whose `_` the inner block's
chain holds, its `|` arm taken for the `else`; and of an `Option`'s or an
enum's, `(None, _) | (_, 1)`, whose `Some` test is the tree's first: each
still MIR's order of arms, `if (a === 1) { return 2; } if (b !== 1 && c
!== 1) { .. } return 2;`.

## Amendment: an alternative tests a tuple's parts in the order written (#632, 2026-11-04)

`(1, _) | (_, 1) => .., (2, 3) | (3, 2) => ..`'s later arm tested `(b
=== 3 && a === 2) || (b === 2 && a === 3)`: MIR tests `b` first there,
past `(_, 1)`'s `b === 1`, and a path's tests were in the order MIR
reads them.

**A path's tests of the parts of one tuple MIR made, `_5[1]`, are in the
order they're written** (`paths::written_order`, before an arm's paths
are factored): by where each test is placed, its alternative's pattern,
`(2, 3)`, then by its part, as a tuple's patterns are written in the
order of its parts. `(a === 2 && b === 3) || (a === 3 && b === 2)`.

Only of a path all of whose tests are of one tuple's parts: a part is an
index of the JS array the tuple is, whose read neither calls nor throws,
so no test guards what another reads, and the same tests are read,
within the path, whatever their order. Any other path keeps MIR's order:
an `Option`'s `x != null` before `x[0]`, a slice's `xs.length === 2`
before `xs[0]`, an enum's `TAG` before its field, a getter's read where
it ran. Of a tree whose guard calls (`Model`), paths stay as the tree
tests them.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. The same pure
reads, the same arm. Held by `test/lowering-matches.test.ts`'s "an
or-pattern's alternatives test a tuple's parts in the order written", a
tuple's, an `Option` of one's and a slice's, run, and the corpus cases
whose JS it reorders, each against native Rust; the mutations
`alternatives-in-mir-order` (`lower/mir/tidy/decisions.ts`) and
`written-order-of-the-alternative-alone`
(`lower/mir/tidy/decisions/paths.ts`).

## Amendment: a match of three values is the chain as written (#673, 2026-11-04)

`(0, 0, 0) => 1, (1, _, _) | (_, 1, _) | (_, _, 1) => 2, (2, 2, _) | (_,
2, 2) => 3, _ => 4` kept MIR's order of arms, its inner block's chain
the `_`'s, the `|` arm the `else`:

```js
if (a === 0 && b === 0 && c === 0) {
  return 1;
}
if (a === 1) {
  return 2;
}
if (b !== 1 && c !== 1) {
  if (a === 2 && b === 2) {
    return 3;
  }
  if (b === 2 && c === 2) {
    return 3;
  }
  return 4;
}
return 2;
```

The inner block's chain had 5 tests to its tree's 4, `(a === 2 && b ===
2) || (a !== 2 && b === 2 && c === 2)`, so it stayed, and the outer
chain, its `else` the `|` arm, the longest, had 14 to the tree's 6. Of
an arm's paths and the chain's arms, each the same pure tests' truth
(`decisions/paths.rs`, `decisions.rs`):

1. **A test of a path whose failing, with the path's other tests, says
   another path of its arm goes** (`absorbed`): `a !== 2 && b === 2 && c
   === 2` beside `a === 2 && b === 2` is `b === 2 && c === 2`, as `p ||
   (!t && q)` is `p || q` where `t && q` says `p`. What it then holds of
   more, the other path holds of, the same arm. Of the paths before it
   first, then those after; a path then alike one before it is once, `p
   || p` being `p`. Not of a test that guards one after it (`guards`):
   of an object the later test reads a part of, `x != null` of `x[0]`'s,
   or a part of what holds one, `shape.TAG` of `shape._0[1]`'s, whose
   failing would read what may throw. A tuple's parts guard none of each
   other, `a[0]` of `a[1]`'s.
2. **What the arms before it in the chain say failing, an arm's paths
   need no test of** (`failing`, `said`): past `a === 1 || b === 1 || c
   === 1`, `a !== 1`, `b !== 1` and `c !== 1` hold wherever the chain
   reads the next arm's test; and paths alike but for one value, each a
   test of it alone, whose union holds of each value it can have there,
   are no test of it (`unions`): `a === 0 || (a !== 0 && a !== 1)` past
   `a !== 1`, `a !== 2 || a === 2` of any.
3. **A path's tests of a value its earlier ones say are none, and a
   later one saying less is what both say, where the first is**
   (`simplified`): `b !== 1` after `b === 0`; `c !== 0 && b === 0 && c
   !== 1`'s `c !== 0 && c !== 1 && b === 0`, its value read there
   already.
4. **Alternatives MIR gives one place, the whole `|`'s, are by the part
   of the tuple each tests last**, the one it names: `a === 1 || b === 1
   || c === 1`.
5. **Where the longest arm's chain has more tests than the tree, each
   other arm is tried as the `else`, the longest first**: of four values,
   `(1, _, _, _) | .. | (_, _, _, 1)`, the `_`.

The rules these take the place of are gone, one rule of each: #412's
failing of an alternative before, `s === A || (s !== A && s === B)`'s
`s !== A`, and of a single test's set, `a === 1 || (a !== 1 && b ===
1)`'s, are rule 1's (`alternatives`' `failed` and `negated`); #631's
second, a path none of whose or-pattern's alternatives holds (`excludes`
in `simplified`), and third, a test the alternatives before it each say,
`X && b !== 1`'s `b !== 1`, are rules 1's and 2's. No test, of the corpus,
its snapshots and the lowering suites, read any of them past these, so
their mutations went with them: `negated-alternative-kept`,
`decisions-alternative-not-absorbed`, `or-pattern-excluding-path-kept`,
`or-pattern-excluding-one-alternative` and `or-pattern-said-atom-kept`.

Only of a tree that tests what a `match`'s patterns name: one testing
what the program wrote, an `if`'s condition or an `&&` or `||`
(`Written::ifs`, `Written::logicals`), stays as the rules before gave it,
for the passes after read those as written, `return (x === y && ..) ||
..` (ADR 0613), `if (on && ..) { .. } else { .. }`; and not of a tree
whose guard calls, whose paths `Model` reads as the tree tests them.

```js
if (a === 0 && b === 0 && c === 0) {
  return 1;
}
if (a === 1 || b === 1 || c === 1) {
  return 2;
}
if ((a === 2 && b === 2) || (b === 2 && c === 2)) {
  return 3;
}
return 4;
```

Soundness: the tests are what the tree reads, each a pure read of the
scrutinee's parts, made once before it; a test goes only where it's
implied, by the path's own tests or the arms the chain has failed, or
where the values it then lets in take the same arm another way; no rule
moves a read past what guards it; and a path goes only where another
alike is kept.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. The same arm for
each input. Held by `test/corpus/or_pattern_three_values.rs` against
native Rust, of three and four values, of `Option`s of tuples whose
`x != null` guards `x[0]`, of slices of tuples whose `length` guards
`xs[1][1]`, and of `f64`s with `NaN`; `or_pattern_paths_absorbed.rs`
re-blessed; `test/lowering-match-paths.test.ts`'s "a match of three
values' or-pattern is the chain as written", run; and the mutations
`absorbed-guard-unread`, `absorbed-failing-says-any-test`,
`absorbed-failing-meets`, `known-of-any-value`, `failing-of-an-and`,
`merged-of-either`, `run-said-of-any` (`lower/mir/tidy/decisions/paths.ts`)
and `twice-of-any-alike-length` (`lower/mir/tidy/decisions.ts`), each a
wrong answer native Rust catches, and `union-of-any-value-alone-kept`,
`else-the-longest-alone`, `alternatives-by-part-reversed` and
`written-tests-read-as-patterns`, the JS. Left: a match whose `Option`'s
or enum's tests MIR makes first, `(None, _) | (_, 1)`, still its order
of tests (#674).
