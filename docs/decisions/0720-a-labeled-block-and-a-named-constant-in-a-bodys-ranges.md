# 0720. A labeled block's `break` and a named constant, in a body's ranges

Status: Accepted. Extends [0576](0576-what-a-bodys-integers-can-be.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): A: a wrap left out is
one that would change nothing.

## Context

React's `shared/ReactPerformanceTrackProperties`, ported, printed one
file's `usize` arithmetic two ways:

```js
// addValueToProperties:                 // addObjectDiffToProperties:
if (indent < 3) {                        } else if (indent < 3) {
  addObjectToProperties(value,             ... addObjectDiffToProperties(
    properties, (indent + 1) >>> 0, ..)        prevValue, nextValue, properties, indent + 1)
addedProperties++;  // a first loop      nextPropertiesChecked = (nextPropertiesChecked + 1) >>> 0;
addedProperties = (addedProperties + 1) >>> 0;  // the next
```

Two gaps in the walk (`body_queries/ranges.rs`) made the difference:

- **A labeled block's `break` was a jump it couldn't follow.** The walk
  takes a `break 'label` to the scope its label names; a labeled block's
  label is the block's own scope, `thir::Block::region_scope`, which it
  never entered, as it entered loops' and expressions'. Lost, it trusted
  nothing it found in the body: `addValueToProperties`'s `'desc: { .. }`
  cost every operator in it its range, the guarded `indent + 1` and the
  count below `OBJECT_WIDTH_LIMIT` too.
- **A `const` of an integer was any value of its type.** `if
  next_properties_checked > OBJECT_WIDTH_LIMIT { break; }` narrowed
  nothing, where `> 100` would have, so the `+= 1` after it wrapped.

## Decision

**A labeled block is a target of its `break`s**, as a loop is: what's
known at each `break 'desc` goes past the block, joined with what's known
where it ends (`block`). **A `const` of an integer, of no generic
arguments, is its value**, rustc's (`constant_integer`), as its literal
is. An operator of constants alone, one of them named, `usize::BITS - 1`,
isn't judged: it's a constant, whose wrap is folded where it's written,
`31`, not `32 - 1`; `STEP * 2`'s wrap stays, as before.

What's left wrapped is what the walk can't bound: the port's `indent + 1`
past `indent < 3 || ..` is of an `indent` of 3 or more, as large as a
caller gives, and `(prevLength - 1) >>> 0` of a length the body lent to
a call that may change it.

## Why

- **The same arithmetic reads the same**, `indent + 1` under `indent <
  3`, wherever the function writes it; a count tested against a named
  limit is a count tested against its value.
- **It's what the walk already does** of a loop's label and a literal;
  nothing new is assumed.

## Consequences

- The port's JS loses the wraps the walk now shows can't happen,
  `indent + 1` under `indent < 3` and both counts, `++`; no corpus case's
  or snapshot's JS changes.

## What holds it

- `test/lowering-numbers.test.ts`, "a labeled block and a named constant
  keep what a body's integers can be": a guarded `indent + 1` in a
  labeled block, a counter tested against a `const`, a `break`'s range
  joined past its block, and a negative `const`.
- Mutations: the block not a target (`labeled-block-not-a-target`), its
  `break`s not joined past it (`labeled-block-breaks-unjoined`), a
  constant unknown (`named-constant-unknown`), an operator of constants
  alone judged (`named-constants-judged`), a negative one read as its
  bits (`named-constant-signed-as-unsigned`).
