# 1074. A block in a condition is a comma expression

Status: Accepted (#523, 2026-11-03). Amended (2026-11-03): a block's
`let` read once, right after it, is read in place. Amended (2026-11-04):
a local's set is a part too, and a block's `let`s are read in from the
last. Amended (2026-11-04, A1, the user's decision): one read twice or
past a call is a `let` declared before the `if`, set in the condition. Extends
[0946](0946-an-if-of-mixed-logicals-is-its-one-test.md), whose `skipped
tests` make an `if`'s one test of the `if`s and `break`s MIR made of its
`&&`, `||` and `!`.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. Each part runs where
and when Rust's does, so the results are the same; the change is to how
the JS reads.

## Context

alien-signals' `computedOper`, ported, writes a block with a statement in
an `if`'s condition:

```rust
if f & 16 != 0
    || (f & 32 != 0
        && (check(f) || {
            flags.set(f & !32);
            false
        }))
{
    value.set(value.get() + 10);
} else if f == 0 { .. }
```

Its JS was labeled blocks and `break`s, labels the Rust doesn't have:

```js
bb20: {
  bb9: {
    if (!(f & 16)) {
      if (f & 32) {
        if (!check(f)) {
          flags.value = (f & ~32) >>> 0;
        } else {
          break bb9;
        }
      }
      ..
```

`skipped tests` reads a tree of `if`s and `break`s as one test (ADR 0946),
and refused one that does anything else on the way. Upstream's JS writes
`(this.flags = flags & ~ReactiveFlags.Pending, false)` in the condition.

## Decision

**What a way through a condition does is done where it is, in a comma
expression of the test**: `if (f & 16 || (f & 32 && (check(f) ||
((flags.value = (f & ~32) >>> 0), false))))`.

- The JS tree has a `Sequence` expression, `(a, b)`, its value the last's.
  Only tidy makes one: `logicals::falls`, which says when a tree of `if`s
  and `break`s goes on past its end, takes a call (an expression
  statement) and a set of a field or an index (an `Assigned`, `(o.f =
  v)`) on a way as done there. The way's truth after them is the
  sequence's last: `(s, false)` where it then leaves, `(s, true)` where it
  goes on, `(s, t)` before a test `t`. Each statement runs exactly where
  it ran: on its way, after the tests before it, before those after it.
  Only what's written in the condition, inside an `&&` or `||` the
  program wrote, as its tests are (`tests_within`): what the `if`'s own
  branch does stays there, `if (a != null && b != null) { log.push(a + b);
  } else { log.push(0); }` of a let chain's `else`
  (`let_chain_else_inside_if.rs`).
- `negated_test` of a sequence negates its value alone, `(s, false)` of
  `(s, true)` (the `!true` it makes is `false` once tested), so `written
  elses` turns the test as it turns any.
- The printer gives oxc a `SequenceExpression`, which it wraps in
  parentheses wherever what's around it binds tighter, as an `||`'s side.
- `js::Writes` takes an `Assigned` in an expression for a write, as an
  assignment statement is, and a closure's where one holds it: the owner
  of what a tree writes knows a sequence's (`an_assignment_in_an_
  expression_writes`, its unit test).

A set of a variable, `{ seen = 5; false }`, is not taken: it sets a
binding, which passes after `skipped tests` read from statements (a `let`
set once is a `const`), so the `if` stays as MIR made it. A `let` in the
block is not taken either. Each is left for a later change that teaches
those passes a binding's write in an expression.

## Why

- **It's how JS writes it**, and how alien-signals does.
- **No label the Rust doesn't have**: a JS label is never an accepted
  difference.
- **One owner**: the tree's truth is still `falls`'; the sequence is its
  answer for a way that does something, not a second recovery of the
  same `if`.

## Alternatives

- **Hoist the statement before the `if`**: it runs only on one way, after
  `check(f)`; hoisting it would run it always, or call `check` twice.
  Rejected.
- **A helper or an arrow called in the condition**: reads worse than
  upstream's comma. Rejected.

## Consequences

- No corpus snapshot or port file changed but the new case's.
- A `Sequence` reaches the walkers (`each`, `visit_vars`, `has_effects`,
  the substitution in `expr_walk.rs`), linking, preparation's scopes and
  printing.

Held by corpus `condition_block_sequence.rs` against native Rust (a field
set and a call on a way, two statements and a test after them, an `&&`'s
block, a value read before the `if`, a `&mut` written through, a block whose
value is `true`, a local's set kept apart, each ways' logs), and mutations
`condition-call-unsequenced`, `condition-field-set-unsequenced`,
`condition-binding-set-sequenced`, `condition-sequence-leaving-goes-on`,
`condition-sequence-of-the-ifs-own` (`logicals/skipped.ts`, moved there with the code, #523), `negated-sequence-whole` (`make.ts`); the `Writes` arm by its unit test, `cargo test`, which the
mutation runner doesn't run.

## Amendment: a block's `let` read once is read where it's read (2026-11-03)

alien-signals' `computedOper` gives its call a block with a `let`:

```rust
|| (flags & PENDING != 0
    && (check_dirty(
        {
            let deps = this.deps.borrow().clone();
            deps.unwrap()
        },
        Rc::clone(this),
    ) || { .. }))
```

`falls` refused the `const deps = this$.deps;` on the way, so the `if`
stayed labeled blocks, `bb56:` and `bb25:`. **A block's `let` on a way
through the condition, read once, by the test right after it, where
nothing JS evaluates before it there does anything, is read there** (`moves::read_again`, the one answer to a
`const` read again in its next statement's place): `if (flags & 16 ||
(flags & 32 && (checkDirty($unwrap(this$.deps), this$) || ((this$.flags
= flags & ~32), false))))`, as upstream writes `checkDirty(this.deps!,
this)`. Case N: the value runs where it ran against all that does
something, and nothing between its `let` and its read did anything.
One read twice, `{ let d = flags.get(); d * d }`, or read past a call,
`more(flags.get(), { let d = bump(flags); d + 1 })`, whose read of
`flags` is held before the call, stays as MIR made it: JS has no
statement in a condition, and those need one. What the `if`'s own
branch does is kept out by its other statements, which `tests_within`
holds to the condition: a `let` on a way is read by a test or by what
the way does, and a way of tests alone, no call or set, does nothing a
`let` could be part of. Corpus `condition_block_sequence.rs`'s `held`,
against native Rust; `lowering-condition-lets.test.ts`, "a condition's
let read twice or past a call stays where it's bound", run, its answers
native Rust's; mutations
`condition-let-kept` and `condition-let-unread`
(`logicals/skipped.ts`, moved there with the code, #523).

## Amendment: a local's set, and `let`s read from the last (2026-11-04)

**A local's set on a way is done where it is, as a field's is**:
`check(f) || { seen = 5; false }` is `if (f & 1 || (f & 2 && (check(log,
f) || ((seen = 5), false)))) { seen += 10; }`. The decision above kept it
out because passes after `skipped tests` read a binding's writes only
from statements. Since ADR 1088 every such question is `js::Writes`',
which counts `(x = ..)` in an expression as it counts `x = ..;`, so the
`let` stays a `let`. Case N: the set runs on its way, after the tests
before it, as Rust's does.

**A block's `let`s are read in from the last**: `more(flags.get(), {
let d = bump(flags); d + 1 })` was `const value = flags.value; const d =
bump(flags); if (.. more(value, d + 1) ..)`. Read from the first, `value`
wasn't read in the statement right after it, `d`'s, so the `if` stayed
labeled. Read from the last, `d` is read in its place first, `more(value,
bump(flags) + 1)`, and then `value`, `more(flags.value, (bump(flags) + 1)
>>> 0)`. `read_again` still holds each one: read once, in the statement
right after it, before anything there does anything, so `flags.value` is
read before `bump`, as JS reads a call's arguments in order. Case N.

And `skipped_tests` reads the blocks a block holds before the block
itself, so one of them that's a test of the block's tree makes the outer
one an `if` too: `bb10: { bb7: { .. } seen += 10; }` is `if (..) { seen
+= 10; }`, not `seen += 10; return seen; } return seen;`, which later
passes made of a `break bb10`.

Corpus `condition_block_sequence.rs`'s `local`, against native Rust;
`lowering-condition-lets.test.ts`, its `pastACall` now read in place and
run; mutations `condition-local-set-unsequenced` (which replaces
`condition-binding-set-sequenced`), `condition-lets-read-first-first` and
`skipped-tests-outer-first` (`logicals/skipped.ts`, moved there with the code, #523).

## Amendment: a block's `let` read twice is a `let` set in the condition (A1, the user's decision, 2026-11-04)

Case ([0262](0262-when-rust-and-js-disagree.md)): N. A block's `let` the
condition can't read in its place, read twice, `more(f * 4, { let d =
flags.get(); d * d })`, or past what does something, kept MIR's labeled
blocks, which a JS label is never an accepted form of. The user chose
among a hoisted `let` set in a comma sequence (A1), the read repeated where
it's a path (A2), and a flag: **A1, a `let` declared before the `if`, set
on its way where the `const` was**:

```js
let d;
if (f & 1 || (f & 2 && ((d = flags.value), more((f * 4) >>> 0, Math.imul(d, d) >>> 0) || ..))) {
```

as React DOM's getModernOffsetsFromPoints declares `let next = null;` and
tests `if ((next = node.firstChild) === null)`
(react-dom-bindings/src/client/ReactDOMSelection.js:76, 96), and
ReactDOMFizzInstructionSetShared.js:490 `let href, precedence, attr,
loadingState, resourceEl, media;` then :515 `if ((resourceEl =
window['$RM'].get(href)))`.

- `falls` reads a block's `const`s from the last; one it can't read in
  its place it makes an assignment where it is, in the way's sequence,
  so it runs where it ran: after the tests before it, before those after
  it, and the `const`s before it are read in place or set before it
  too. The `let`s are `Hoisting`'s, declared before the `if` by
  `skipped_test`. A body's names are its own till tidy is done, each
  binding's apart from every other, `d` and `d$1` (ADR 0352 reclaims them
  after), so a `let` declared a block out shadows nothing: the parameter
  `d` the branch reads stays the parameter.
- A `let` a block it held declared before its test (`bb12: { let d$1;
  if (..) { break bb12; } n += d; }`) is declared before the outer one's
  too, so the outer block is one `if` as well.
- `skipped_tests` reads the blocks a block holds first, so one that's a
  test of the outer's tree makes the outer an `if` (above, step 2).
- **A last resort, late**: the `let`s are made only by the pass
  `condition lets`, first in tidy's `finish`, of a block MIR made still
  there once every other pass is done, as `skipped_tests` and
  `chains_in_blocks` make it with `lets`. Their first run, early, makes
  none: a block a later pass makes another form of, a getter's `const kind
  = input.kind; if (kind === 2 || kind === 1)` that returns, or a
  `while`'s test, stays that form, which reads better than a `let`
  set in a sequence. One the jumps made a `return` of before the late
  pass may read as the test's negation, `if (!(f & 1) && ..) { return n; }
  n += d;`. Not where a
  loop's body ends in the block (`continued`'s, which makes `continue`s of
  it, no label left): there `skipped_test` is given no list to declare in.
- `shadows`' `set_after_last_read` takes a test that sets the shadow as
  setting it before either branch, and refuses one that also reads the
  binding it shadows: before it, `let d$1;` set in a test was taken for a
  shadow of the parameter `d` its branch then read (`shadows` merges a
  name given apart into its binding where that's read no more), a wrong
  answer of A1's that native comparison caught (corpus `twice`).

Held by corpus `condition_block_sequence.rs`'s `twice` (a parameter of the
`let`'s name the branch reads) and `beside`, and `lazy_effects.rs`'s `if t
|| bump(&mut y)`, against native Rust; `lowering-condition-lets.test.ts`'s
`readTwice`, run; `lowering-or-patterns.test.ts`, a getter's chain read
once and left as it was; mutations `condition-let-never-hoisted`,
`condition-let-of-a-held-block-kept` (`logicals/skipped.ts`, moved there with the code, #523), `condition-lets-early`
(`stages.ts`) and `shadow-set-in-a-test-unseen` (`shadows.ts`).
