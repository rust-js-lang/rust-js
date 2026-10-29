# 0683. An `if` keeps its test and branches as written, and an `else if` stays a chain

Status: Accepted. Amends [0237](0237-no-else-after-leaving.md), whose
`else` after a branch that leaves goes only where each arm before it
leaves too; extends [0575](0575-an-if-keeps-its-else-and-a-tail-if-is-a-conditional.md)
and [0364](0364-mir-lowering.md)'s tidy. Amended below: an `||`'s block
that gives one value is the conditional (#413); and an `if` of `||` whose
branch jumps is one `if` (rd225).

Case ([0262](0262-when-rust-and-js-disagree.md)): N. Each rewrite changes
how the JS reads, not what runs.

## Context

React's `describeObjectForErrorMessage` (shared/ReactSerializationErrors.js),
ported, read worse than its original twice:

```js
// The original:                          // rust-js's:
if (__DEV__ && jsxChildrenParents.has(o)) {   if (!__DEV__ || !jsxChildrenParents.has(o)) {
  // Print JSX Children                         // Print Array
  ..                                            ..
} else {                                      } else {
  // Print Array                                // Print JSX Children
  ..                                            ..
}                                             }

if (o.$$typeof === REACT_ELEMENT_TYPE) {      if (o.$$typeof === REACT_ELEMENT_TYPE) {
  str = ..;                                     str = ..;
} else if (o.$$typeof === CLIENT_REF) {       } else {
  return describeClientReference(o);            if (o.$$typeof === CLIENT_REF) {
} else if (__DEV__ && ..) {                       return describeClientReference(o);
  ..                                            }
} else {                                        if (__DEV__ && ..) { .. } else { .. }
  ..                                          }
}
```

MIR jumps from each test of an `&&` to the `else`, so the `then` is the
labeled block's and the `else` follows it: `l: { if (a) { if (b) { s;
break l; } } t }`. Tidy wrote that `if (!(a && b)) { t } else { s }`,
counting on a later pass to turn it back, which none does where a branch
ends in a loop. And preparation's rule that an `if` whose branch leaves
has no `else` (ADR 0237) took the `else` of an `else if` too, whose
statements were then the outer `else`'s: the chain broken.

## Decision

**An `if` of an `&&` MIR made a block of is as the program wrote it**:
`if (a && b) { s } else { t }` where `s` is written first, `if (a || b) {
t } else { s }`, its tests negated, where `t` is. Which is first is where
it's written (`first_written`), as a conditional's branches already are.

**An `else if` of an arm that goes on stays an `else if`**: `if (a) { x =
1; } else if (b) { return; } else { .. }` keeps its chain. An arm that
leaves loses its `else` only where every arm before it leaves too, the
chain's `if`s one after the other, as before: what ESLint's
`no-else-return` asks, which allows an `else if` after an arm that goes
on.

## Why

- **It's how it's written**, in the Rust and in the original JS: React's
  own `&&` test first, its `else if` chain of four arms.
- **One rule of which branch is first**: by where the program wrote it,
  as `branch_values` orders a conditional's and `written_elses` an `if`
  MIR made a guard of; not a negation a later pass must undo.
- **A chain read as one**: breaking it puts an arm's `if` in an `else`
  beside the next arm's, which reads as nested code, not a choice.

## Alternatives

- **Turn `if (!a || !b) { t } else { s }` back in a later pass**: the
  turn and its undoing, two rules for one question. Rejected.
- **Keep every `else` the program wrote, after a `return` too**: React and
  ESLint write `if (c) { return a; } return b;`, and ADR 0237's reasons
  hold. Rejected.

## Consequences

- No corpus case's or snapshot's JS changed: none had either shape.
- Preparation's `unelsed` holds the `else` lists of a kept chain as it
  walks the lists from the outside in.

## What holds it

- `test/lowering-written-branches.test.ts`, "an if of an && keeps its test
  and its branches' order, each a loop", and "an else-if chain keeps its
  else past an arm that returns", with a chain each of whose arms
  returns, unchanged; each run.
- Mutations: `mir-tidy-and-chain-turned`, `else-if-unelsed`,
  `else-if-links-unheld`.

## Amendment: an `||`'s block that gives one value is the conditional (#413)

React Float's hydrateHoistable gives `getAttribute` a value `if`,
`node.getAttribute("href") !== (href == null || props.href === "" ? null
: props.href)` upstream, and so does rust-js where the loop is plain. In
a loop over a `RefCell`'s borrow, one `try` around the loop (ADR 0437,
rd200), the second test was a `const` read once, `const _41 =
props.href === "";`, made of each step's `try`, until `in place` read it
where it is; by then the `jumps again` run of `branch values` that folds
MIR's block, `l: { if (a) { if (!b) { x = v; break l; } } x = null; }`,
into `x = a && !b ? v : null` had passed, and `in place`'s run made the
block the `if` above, `if (href == null || ..) { x = null; } else { x =
v; }`, which nothing made one value again: `let unknown; if (..) { ..
}`.

**Where `branch values` makes such a block an `if` with an `else`, it
runs again on the list** (`branches.rs`'s `unlabeled`), so the `if`'s
branches that give one variable its value are the conditional, as any
other `if`'s are (ADR 0657): `x = a || b ? null : v;`, read in place.
What another run changes is only what one run changes of an `if` with an
`else`, which a later run of the stage would see anyway.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the same tests in
the same order, the same value. Held by corpus case
`borrowed_loop_value_if` against native Rust and mutation
`unlabeled-chain-not-a-value` (`mir/tidy/branches.ts`). Other values MIR
made such a block of are their conditionals now too, as the rule says of
a value: the scheduler's `const startTime = delay != null && delay > 0 ?
currentTime + delay : currentTime;` (its snapshot, a guarded `match`'s
value, was `let startTime; if (delay == null || !(delay > 0)) { .. } else
{ .. }`), `lowering-loops`' `const substr = .. ? "{...}" : String(value);`
and `lowering-names`' `const n = x != null && x > 3 ? x : 0;`.

## Amendment: an `if` of `||` whose branch jumps is one `if` (rd225, #413)

hydrateHoistable's link and meta cases go on to the next node where an
attribute differs, `if !a(..) || !b(..) || !c(..) || !d(..) { continue;
}`, upstream's one `if (a !== (..) || b !== (..) || ..) { continue; }`.
MIR jumps to the one `continue` from each side, and in a loop over a
borrow, whose jumps settle only at `finish` (rd200's one `try`), what was
left was an `if (..) { continue; }` of each side.

**`if (a) { J } if (b) { J }`, of a jump alike, a `continue`, a `break`
or a `return;`, where `a` and `b` are tests of one `if` condition the
program wrote, is `if (a || b) { J }`** (`logicals.rs`'s `written_ors`,
`finish`'s `written ors`, after `written jumps` settles them): `b` is
tested once `a` is false, as `||` tests it. A test is of the smallest
condition of `Written::ifs` that holds it, or that it holds ending where
it ends, MIR's test of a whole `if`; two `if`s written apart, `if r !=
rel { continue; } if t != title { continue; }`, are each of their own
and stay two.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the same tests in
the same order. Held by `test/lowering-cell-borrows.test.ts`, "a condition
of value ifs written with || in a borrow's loop is one if", run in Node;
corpus case `borrowed_loop_value_if` (`apart`, two `if`s) against native
Rust; mutations `written-ors-kept-apart`, `written-ors-of-any-ifs`
(`mir/tidy/logicals.ts`). The jump alike in each is checked but has no
mutation: MIR makes no two `if`s of one condition jump apart.
