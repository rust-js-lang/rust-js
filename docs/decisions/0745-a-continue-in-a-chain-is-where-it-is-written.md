# 0745. A `continue` in a chain of `if`s is where it's written

Status: Accepted. Extends [0364](0364-mir-lowering.md)'s jumps past blocks
and branch ends. Amended (2026-10-28): `jumps into branches` owns the rule
`breaks as ends` asked for too, one authority. Amended by
[0702](0702-a-while-is-its-test-and-its-break-where-written.md) (uz2): a
jump the program wrote after an `if` it wrote with an `else` stays there.

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

## Amendment: each arm of an `else if` chain that goes on has the jump

React's commitBeforeMutationEffects_begin ends two arms of an `else if`
chain with a `continue` and falls out of the chain's end to the loop
body's rest:

```rust
if is_hidden {
    ..;
    continue;
} else if current.as_ref().is_some_and(|c| c.lanes.get() == 7) {
    ..;
    continue;
}
```

MIR joins the two `continue`s after the chain, and its fall-through is a
`break` past them: `bb107: { if (isHidden) { .. } else if (..) { .. } else
{ break bb107; } continue; }`. `jumps into branches` moved a jump only
into one of two branches, the `if`'s `then` or its `else` whole, and the
`else` here, the chain's rest, doesn't leave as a whole. **Of a chain only
whose last `else` leaves, each arm before it has the jump**, `if (isHidden)
{ ..; continue; } else if (..) { ..; continue; }`, and `branch ends` then
drops the `else { break bb107; }` and its label. A chain of arms that leave
each their own way, as a `match`'s guarded arms make, `if (tag === 3) { if
(..) { break bb6; } } else { if (tag === 5) { break bb6; } else { break
bb25; } } break bb25;`, stays as it is, so `switches` reads its cases
(`test/lowering-switch-fallthrough.test.ts`).

Case ([0262](0262-when-rust-and-js-disagree.md)): none observable; a jump
moves to each place that goes on to it.

Held by the corpus case `continue_arms_of_else_if` (native-compared) and
the mutation `jumps-into-chain-arms-uncounted`. Its second,
`jumps-into-arms-that-leave-their-own-way`, went once no program reached
what it changed (ADR 1004, amended).

## Amendment: one owner gives a chain's arms the jump after it (2026-10-28)

Two passes put a jump MIR shared after an `else if` chain into each arm
that goes on: `jumps into branches` (`tidy/jumps/continues.rs`, this
decision), of a jump after the chain, and `breaks as ends`
(`tidy/jumps/labels.rs`, 041187f13, [0702](0702-a-while-is-its-test-and-its-break-where-written.md), amended, React's
commitBeforeMutationEffects_begin), of a `continue` after a chain whose
last `else` is `break l;` where `l` ends. Each had its own walk of the
chain and its own test of which arms go on: one rule, two authorities
(AGENTS.md's "one way, one authority").

**`jumps into branches` is the rule's one owner, and `breaks as ends` is
gone.** With a `match`'s last arm its `else`
([0237](0237-no-else-after-leaving.md), amended), the chain
commitBeforeMutationEffects_begin's port makes reaches `jumps into
branches` with its `continue` after it, which gives each arm its own, and
`tail jumps` drops the `break l;` at `l`'s end: with `breaks as ends` run
nowhere, no snapshot of the corpus, the guide or the React ports changed.
It covers each case the other did, so it stays and the other goes.

Held by the architecture's table of owners, which names
`jumps_into_branches`, and the mutation `jumps-into-branches-off`, whose
tests are both rules' (`lowering-early-exits`' chain, the corpus's
`continues_in_arms`, against its snapshot).

## Amendment: a block's `break` in a loop inside it is the labeled loop's `continue` (#442)

`jumps past blocks` makes each `break l` of a block MIR made the jump
after it, `l: { .. break l; .. } continue;`, but not one in a loop inside
the block, where `continue;` would go on with that loop. So `'outer: for
&x in items { if !keep(x) { continue; } while j < 3 { if j == x {
continue 'outer; } .. } total += 100; }` kept MIR's block and its
`break`, `outer: for (..) { bb25: { if (keep(x)) { for (..) { if (j ===
x) { break bb25; } .. } .. continue; } } continue; }`.

**In the turn of a loop the program labeled, a block followed by its
`continue` has each `break` of it in a loop inside it `continue outer`**
(`continues.rs`' `named_continues`), which goes on from the same place:
`if (!keep(x)) { continue; } for (..) { if (j === x) { continue outer; }
.. } total += 100;`, as written. A loop the program didn't label has no
name to give, and keeps the block. Case ([0262](0262-when-rust-and-js-disagree.md)):
N, the same turn goes on, the same `finally`s run.

Held by `test/corpus/continue_outer_past_block.rs` and
`borrow_loop_inner_break_outer.rs` against native Rust, and mutations
`named-continues-unseen` and `named-continue-unnamed`
(`tidy/jumps/continues.ts`).
