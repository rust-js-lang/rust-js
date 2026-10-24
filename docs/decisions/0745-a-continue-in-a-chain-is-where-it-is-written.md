# 0745. A `continue` in a chain of `if`s is where it's written

Status: Accepted. Extends [0364](0364-mir-lowering.md)'s jumps past blocks
and branch ends.

Case ([0262](0262-when-rust-and-js-disagree.md)): it can't be observed:
each rewrite moves a jump to where it goes on from anyway, or an `if`'s
`else` to what follows an `if` whose branch never goes on.

## Context

React's `addObjectDiffToProperties` (shared/ReactPerformanceTrackProperties.js)
ends arms of a chain of `if`s with a `continue`:

```rust
if prev_value != next_value {
    if indent == 0 && key == "children" {
        ..;
        continue;
    }
    if indent >= 3 {
        // ..
    } else if let Some(prev_object) = prev_value /* && .. */ {
        if /* .. */ { ..; continue; }
        // ..
        if !nested_equal { .. } else if prev_length == properties.len() { .. }
        continue;
    } else if /* .. */ {
        ..;
        continue;
    }
    add_value_to_properties(..);
}
```

MIR sends each `continue` to one block, which goes to the loop's step.
Tidy made a JS block of each place they meet, `bb294: { .. break bb294;
.. } continue;`, and `bb257: { .. break bb257; .. }` of the chain's
fall-through, where upstream writes an `if`/`else if` chain whose arms end
in `continue;`. `jumps past blocks` rewrote a block's `break`s to the jump
after it only where that jump named a label, and by then `innermost jumps`
had made the loop's `continue bb3` a `continue`.

## Decision

- **A block MIR made followed by a jump, `l: { .. break l; .. } continue;`,
  has each `break l` that jump**, of no label too: not in a loop inside the
  block, where one of no label is the inner loop's. Not a block the program
  labeled, `'sum: { .. }`, which keeps its name ([0742](0742-a-label-the-program-wrote-is-its-name.md)).
- **An `if` one branch of which never goes on, followed by a jump, `if (c)
  { a } else { b; break l; } continue;`, has the jump where the other branch
  ends**, `if (c) { a; continue; } else { b; break l; }`, where it goes on
  from (`jumps into branches`).
- **`branch ends` runs again once those have their shape**, and of a block's
  `if (c) { ..; continue; } t` whose `then` breaks out to the block's end
  inside, `if (c) { .. } else { t }`, its `then` then ending where the
  block does, to be had so too. `unelsed` (ADR 0237) writes no `else`
  where the `then` still never goes on.
- `branch ends` leaves in its place a block the program labeled that ends
  another, `'desc: { .. }`, rather than merging it into the one it ends.

## Consequences

- The port's loop body is upstream's: `if (indent === 0 && key ===
  'children') { ..; continue; }`, the chain of `if`s with each arm's
  `continue`, `if (!nestedEqual) { .. } else if (..) { .. } continue;`, and
  no label.
- An arm of two values in a `switch`'s case, `if (typeof name === "string"
  && name) { desc = ..; } else { desc = "() => {}"; } break;`, is an `if`
  and its `else`, as upstream's `addValueToProperties`.
- `while (true) { bb49: { if (n < 2) { .. continue; } } break; }` of
  `while n < 2 && (..)` is `while (n < 2) { .. }` (`lazy_effects`); a
  `match`'s arms that MIR's blocks held apart are one `else if` chain
  (`calculator`, `returns_in_place`, the inventory snapshot). The corpus
  has 13 labeled blocks, not 17. With fewer lines, the share of renamed
  `x$1`s grows, 761.2 to 761.4 in 10,000 lines, of as many as before.
- A block's `break` in a loop inside it, followed by a jump of no label,
  stays: no program the corpus or the tests have makes one, so no mutation
  holds that guard; it's what JS's `continue;` there would mean.

## Amendment: a written `continue` its rest made an `else` stays

`jumps into branches` puts what goes on after `if (c) { a; continue; }`
in its `else` where the `if`'s other branch leaves, so the `if` is the
loop body's last, and `tail continues` took its `continue`, the program's,
for one where the body ends anyway: `if (key in next) { checked++; } else
{ .. }`, where it wrote `if (key in next) { continue; }` (`test/js-object.test.ts`'s
"a for loop of keys_in"). **The `if` of a `continue` the program wrote,
`tests_continue`'s, has what goes on after it after it again**, its
`continue` kept: `if (c) { a; continue; } b`, as written. `mut_ref_loop.rs`
reads as its Rust does too. Held by the mutation `written-continue-elsed`.
