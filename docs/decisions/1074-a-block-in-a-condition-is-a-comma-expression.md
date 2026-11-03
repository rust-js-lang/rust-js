# 1074. A block in a condition is a comma expression

Status: Accepted (#523, 2026-11-03). Extends
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
`condition-sequence-of-the-ifs-own` (`logicals.ts`), `negated-sequence-whole` (`make.ts`); the `Writes` arm by its unit test, `cargo test`, which the
mutation runner doesn't run.
