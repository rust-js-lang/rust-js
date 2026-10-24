# 0755. A `return` a block MIR made leads to is each of its `break`s

Status: Accepted. Extends [0745](0745-a-continue-in-a-chain-is-where-it-is-written.md),
a block's jumps to the one after it, to a `return` after it, and
[0754](0754-a-return-arms-go-on-to-is-in-each.md) to the branches of the
`if` a block ends with.

Case ([0262](0262-when-rust-and-js-disagree.md)): it can't be observed:
where a `return` is written, and a test no path needs, is how the JS reads,
not what it does.

## Context

React's `addValueToProperties` (shared/ReactPerformanceTrackProperties.js)
returns from its Promise case where each arm ends:

```js
if (objectName === 'Promise') {
  if (value.status === 'fulfilled') {
    // ..
    if (properties.length > idx) {
      // ..
      return;
    }
  } else if (value.status === 'rejected') {
    // ..
  }
  properties.push([..]);
  return;
}
```

Its port's `return`s are past the drop of `object_name`, a `String`, so
MIR jumps to one block for each, then returns. The JS kept two blocks MIR
made, and the `typeof` the status's `match` tests first:

```js
bb390: {
  // ..
  bb272: {
    if (typeof status === "string") {
      if (status === "fulfilled") {
        // ..
        if (properties.length > idx) {
          // ..
        } else {
          break bb272;
        }
      } else if (status === "rejected") {
        // ..
      } else {
        break bb272;
      }
      break bb390;
    }
  }
  properties.push([..]);
}
return;
```

`jumps past blocks` made each `break` of a block followed by a `continue`
or a `break` that jump, but not one followed by a `return`. And a chain of
a string's literals has no `typeof` around it only where the `if` of the
`typeof` holds the chain alone (`branch_values`), not a `break` after it.

## Decision

- **A block MIR made followed by a `return` of one value, or of none, is
  that `return` at each of its `break`s** (`jumps past blocks`), which went
  on to it: a `try`'s too, as `return` runs its `finally` as the `break`
  did. Not a `break` where the block ends anyway, gone first, so the
  `return` after it is written once, not again in an arm; nor one in a loop,
  which may be the loop's way out, `blocks loops end`'s, its test the
  loop's `while`. The value is a constant or a name,
  `returns_one_value`, the one answer ADR 0754's `return` asks too.
- **A block's `return` that some paths of the `if` before it go on to
  (ADR 0754) is put in each where the two end a branch of the `if` the
  block ends with**, which ends where the block does: `bb272: { if
  (typeof status === "string") { if (..) { .. } else { break bb272; }
  return; } }` is `if (typeof status === "string") { if (..) { ..; return;
  } }`, and a branch left with nothing is none.
- **A chain those leave guarded by what each of its tests says is the
  chain alone** (`guarded chains`, the rule `branch_values` asked, now a
  pass of its own it asks too): `typeof s === "string"` is said by `s ===
  "fulfilled"`, as `===` of a string literal is false of any other value.

The Promise case is upstream's:

```js
if (objectName === "Promise") {
  const status = value.status;
  if (status === "fulfilled") {
    // ..
    if (properties.length > idx) {
      // ..
      return;
    }
  } else if (status === "rejected") {
    // ..
  }
  properties.push([..]);
} else {
```

## Alternatives

- **The `return` where MIR's block returns, in `structure.rs`**: the block
  drops `object_name` first, which is nothing in JS only once drops are
  lowered; tidy sees what's left.
- **A `typeof` left out of `&&` alone**: the port's `typeof` isn't in an
  `&&` but an `if` around the chain, which the guarded chain's rule is the
  owner of.

## Consequences

- The port's `addValueToProperties` has no block MIR made but `desc`'s.
- `option_combinator_drops`' `parsePair` has no block MIR made either, its
  `return result;` in the two branches that go on to it.
- Held by `test/lowering-labels.test.ts`'s "a block MIR made followed by a
  return is each break's return", `test/port-performance-track.test.ts`'s
  "addValueToProperties' Promise case returns where it's written", and the
  mutations `jumps-past-blocks-returns-kept`,
  `jumps-past-blocks-return-at-tails`, `jumps-past-blocks-return-into-loops`,
  `returns-into-branches-at-ends-not`, `returns-into-branches-else-kept`
  and `guarded-chains-late-off`.
