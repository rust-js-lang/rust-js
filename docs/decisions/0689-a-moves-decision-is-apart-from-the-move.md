# 0689. A move's decision is apart from the move

Status: Accepted. Extended by [0750](0750-a-temporary-is-read-past-a-method-and-into-a-block.md): past a method, and into a block.

Case ([0262](0262-when-rust-and-js-disagree.md)): none; the JS is the
same, byte for byte, across the corpus.

## Context

Tidy reads a temporary MIR made, `const _5 = e;`, where it's read, `g(e)`,
when that keeps what the program does. Whether it does takes six facts:

- that the name is a temporary, or an element made ahead of a branch;
- whether `e` does something;
- whether a call can change what it reads;
- whether a statement between assigns what it reads;
- whether it's read once, and in what part of the statement;
- whether all JS evaluates before it there passes it (`leads`), a JSX
  `key` and a test it was made under among them.

A review of the compiler's boundaries (2026-10-23) found `inlined`, which
asked them, the hardest function it read. The facts were asked between
the rewrite's own steps, in one loop that also found the statement,
changed it and went back one. So a reviewer explained an accepted move by
tracing the whole loop. "Reading this has no effects" isn't "reading this
later gives the same": `const saved = object.value; change(object);
consume(saved);` must stay, and nothing said why it did.

## Decision

**`tidy/moves.rs`'s `moved` decides, of the tree as it is, where a
temporary's value may be read in its place and why. `inlined` applies
what it says, and nothing else.**

- **`moved(stmt, rest, facts)`** answers `(Moved { at, read, replaced },
  Why)`, or nothing:
  - `at` is the statement that reads it.
  - `read` is which of that statement's expressions does: its value, an
    assignment's target whose object it is, or an `if`'s test.
  - `replaced` is that expression with the value in it.
  - `Why` is `Stable`, when it does nothing and nothing it reads changes,
    or `ReadFirst`, when everything evaluated before it there passes it.
- **Each fact is still its own**: effects, what a call can change, what a
  statement between assigns, and the order JS evaluates in. One
  `is_safe` would hide the distinctions the answer depends on. The body's
  facts are one `MoveFacts`, asked of the body as it is when the pass
  runs (ADR 0670).
- **Unit tests pin the decisions**, each with its `Why`:
  - `object.field` read past `change(object)` stays.
  - `a + 1` is stable.
  - `g(f())` moves.
  - `g(h(), f())` stays.
  - A call past a statement that acts stays.
  - A name the program wrote stays.

## Consequences

- An accepted move is explained by `moved` and its inputs. A new fact is
  added to `moved`, and a test of its `Why` beside it.
- `tidy/bindings.rs`, which held it, keeps the declarations, aliases and
  temporaries' names, under its cap.

## Amendment: two arms as one assignment, decided apart too

A second review (2026-10-24) found `branch_values` as `inlined` was: the
arms' shape, why they may be one value, which reads better, which comes
first and the rewrite, one guard of one `match` arm. **`one_assignment`
decides whether `if (t) { x = a; } else { x = b; }` is `x = t ? a : b;`,
and gives what that takes**: the target, each arm's value placed where its
arm is, and whether the `else`'s comes first. `branch_values` writes it,
and nothing else. The unit test `one_assignment_of_two_arms` pins each
reason beside the case it refuses: two variables, an arm that does more,
each adding to it, a conditional of more branches but a temporary's, the
function's result with an arm of nothing, and a `switch`'s arms; and a
test that does something, which is evaluated once either way. The JS is
the same, byte for byte.
