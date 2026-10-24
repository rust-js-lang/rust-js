# 0790. A `break` the program wrote is where it's written, a case's `break;` where its block ends a case

Status: Accepted. Amends [0742](0742-a-label-the-program-wrote-is-its-name.md),
whose labeled block tidy could make something else, and extends
[0636](0636-a-guarded-arm-is-a-case.md)'s guarded case and
[0668](0668-a-match-of-what-a-value-is-is-a-switch-of-its-typeof.md)'s
`switch (typeof value)` to a guard that holds.

Case ([0262](0262-when-rust-and-js-disagree.md)): it can't be observed:
where a `break` is written, whether a block has a label, and a test that
always passes are how the JS reads, not what it does.

## Context

React's `addValueToProperties` (shared/ReactPerformanceTrackProperties.js)
is a `switch (typeof value)` whose `case 'object'` leaves early, each way
it describes the value:

```js
case 'object':
  if (value === null) { desc = 'null'; break; } else {
    if (readReactElementTypeof(value) === REACT_ELEMENT_TYPE) {
      // ..
      if (key == null && propsLength === 0) {
        desc = '<' + typeName + ' />';
        break;
      }
      // ..
      return;
    }
    // ..
    if (ArrayBuffer.isView(value)) {
      desc = typeof length === 'number' ? .. : objectName;
      break;
    }
```

Its port wrote an `if let` of the object's kind, a labeled block in it,
and a `match` of the rest in its `else`. The JS read `if (value != null &&
typeof value === "object") { .. } else if (value === null) { .. } else {
switch (typeof value) { .. } }`, a `desc: { .. }` label in it, and the
JSX branch's `break 'desc`s as negated, nested `if`s. Three things kept
it from upstream's shape:

- **The breaks the program wrote had no span**, so tidy couldn't tell them
  from MIR's: `tail jumps` and `branch ends` made each an `else`, and the
  block MIR made past the drops at its end, `bb59`, named its own breaks.
- **A guarded arm wasn't a case.** Written as Rust's one `match` of what
  the value is, the object arm needs the value bound, `Kind::Object(_) |
  Kind::Array(_) if let Some(value) = value`, and a guard of an or-pattern
  kept the `match` an `if` chain, `if (value != null)` in each.
- **No `break` of a case's.** A block that ends a case leaves where the
  case does, but kept its label.

## Decision

- **The port is one `match js::classify(value)`**, as upstream is one
  `switch`: `Kind::Null` first, then `Kind::Object(_) | Kind::Array(_) if
  let Some(value) = value => 'desc: { .. }`, then the rest.
- **A guard that holds tests nothing** (`body_queries::held_guards`, the
  one answer): `if let Some(v) = value` of an arm of `match
  js::classify(value)` whose variants are none of `undefined` and `null`,
  of an enum with both, of a `value` the body never changes. A binding
  that's its argument says a `None` is `undefined` or `null` alone, so the
  arm matched says it's a `Some`. Then `hints::written_switches` takes the
  arm as one without a guard; MIR lowering goes from each alternative past
  the guard's test to the arm (`mir_held_arms`, the edges `cfg::Graph`
  follows), and reads what the guard binds as the value it's the `Some`
  of, `value`, not `value$1` (`mir_held_bindings`). A guard that may fail,
  of an arm of `undefined` or of a `value` the body sets, is tested as
  before.
- **A `break` the program wrote of a block it labeled has the span where
  it's written** (`structure.rs::written_break`), and of a `break 'desc
  match .. { .. }`, the jump from where its arms meet, not each arm's way
  there (`goes_on_to_break`). That span is the one fact the rest read:
  - `early_exit` writes it first as it writes a `return`, the program's
    branch first;
  - tidy names the block first (`written_blocks`, the first of `jumps`,
    which `named_labels` did late), a `break` of a block MIR made past the
    drops at its end the block's own; then each pass that reads a block
    MIR made (`is_block_label`) leaves it, and `drop_tail` keeps a written
    `break` at its end;
  - an `||` the program wrote whose branch is such a `break` is as written,
    `if (a || b) { desc = ..; break; } r` (`or_blocks`), not `if (!a &&
    !b) { r } desc = ..`.
- **A block a case ends with, through the `if`s it ends, leaves where the
  case does: each of its `break`s is the case's `break;`, and its label
  goes** (`switches::blocks_ending_cases`), as `breaks_past_switches` has
  a block's `break` in a `switch` it ends with (ADR 0731).

The port reads as upstream:

```js
switch (typeof value) {
  case "object":
    if (value === null) {
      desc = "null";
    } else if (readReactElementTypeof(value) === REACT_ELEMENT_TYPE) {
      // ..
      if (key == null && propsLength === 0) {
        desc = "<" + typeName + " />";
        break;
      }
      // ..
    } else {
      // ..
      if (ArrayBuffer.isView(value)) {
        // ..
        desc = typeof length === "number" ? `${objectName}(${length})` : objectName;
        break;
      }
```

No label is left in it.

## Where a label is still needed

A block the program labeled that ends no case, and whose end isn't where a
loop's or a `switch`'s own `break` lands, keeps its label, by the name the
program gave it (ADR 0742), its `break`s where they're written:
`labeled_blocks`' `'inner: { if (..) { break 'inner; } .. }` is `inner: {
if (log.length === 1) { break inner; } .. }`, as written, not the negated
`if` it was. JS has no other way to leave a block.

## Alternatives

- **The port's `if let`, its `else` a `switch`, fused into one `switch`**:
  a rewrite of two constructs the program wrote apart into one it didn't.
  Rejected for the `match` that's upstream's `switch`, which Rust writes
  as idiomatically.
- **A guarded or-pattern as a case whose statements test the guard** (ADR
  0636): MIR tests each alternative with a guard of its own, and a guard
  that fails goes on to `_`'s arm, which a case can't. A guard that holds
  has neither.
- **Undo `else`s into `break`s late**: tidy has no record of which `else`
  was a `break`; the span is the record.

## Consequences

- The port's `addValueToProperties` is a `switch` with no label; the
  `labeled_blocks` corpus case keeps its `break`s where written.
- Not yet upstream's: the JSX branch's second test, `indent < 3 ||
  (propsLength === 1 && propsKeys[0] === 'children' && key == null)`, of
  three parts, still reads negated, `if (indent >= 3 && (..)) { ..;
  return; } desc = ..; break;`, as `or_blocks` reads an `||` of two parts;
  and upstream's `if (value === null) { desc = 'null'; break; } else {`
  is `if (value === null) { desc = "null"; } else if (..)`. A chain the
  case's `if (value === null)` begins takes the `if` after it as an `else
  if`, so a `break;` ending its branch is followed by an `else`, `} else
  if ("name" in value) { ..; break; } else { .. }` (the guide's
  `describe`), where none is needed.
- Held by `test/lowering-labels.test.ts`'s "a block the program labeled
  that ends a case is the case's breaks" and "a guard of what a kind says
  is there is tested where it may fail", and
  `test/port-performance-track.test.ts`'s "addValueToProperties is a switch
  whose case breaks where it's written", and mutations of each rule.
