# 0742. A block's label the program wrote is its name in JS

Status: Accepted. Keeps [0158](0158-labeled-blocks.md), a labeled block
is JS's labeled block, under MIR's lowering. Amended by
[0790](0790-a-break-the-program-wrote-is-where-it-is-written.md): the
block is named first, its `break`s are where they're written, never an
`else` or a `return` tidy makes of them, and a block that ends a case is
the case's `break;`s, no label. Amended by
[0944](0944-a-label-the-program-wrote-labels-the-switch.md): the name is
camelCase, `getTag`.

Case ([0262](0262-when-rust-and-js-disagree.md)): it can't be observed:
a label's name is how the JS reads, not what it does.

## Context

React's `addValueToProperties` (shared/ReactPerformanceTrackProperties.js)
gives its description from a labeled block, leaving it from deep inside:

```rust
let desc = if let Some(value) = value && .. {
    'desc: {
        ..
        if js::array_buffer::is_view(view) {
            break 'desc format!("{object_name}({length})");
        }
        ..
        return;
    }
} else { .. };
```

ADR 0158 wrote `'desc: { .. }` as `desc: { .. }`. MIR has no labels: it
jumps to the block where the labeled block ends, and the structure made of
MIR names each block a jump leaves by that block's number. The JS read
`bb316: { .. break bb316; .. }`, a name nobody wrote. A loop the program
labeled kept its label already, by where the loop is written (`named_labels`).

## Decision

**A JS block MIR's jumps leave, that a `break` the program wrote of a
block it labeled goes to the end of, is named by the program's label**:
`desc: { .. desc = ..; break desc; .. }`. The jump from where the `break`
is written, back through the drops on its way, is that `break`'s
(`structure.rs`, as a `return` the program wrote is found); the block it
leaves is given the labeled block's span, and tidy names it as it names a
loop the program labeled, where no label around it has that name. Of
blocks its `break`s leave, past the drops at its end, the one around the
others has the name.

Where tidy makes the labeled block something else, a `return` in place
of each `break` or an `else`, no label is left to name, as before.

Whether a label is MIR's is asked of `is_block_label` alone: tidy's
decisions had a copy of its test, which the architecture test now holds
out.

## Consequences

- The port's `bb316` is `desc`. Its other labels are MIR's, of an `||` or
  a `match`, and still a block's number.
- The labeled blocks corpus case's `bb23` and `bb62` are `search` and
  `sum`, and a comment before `let total = 'sum: {` is kept, now that the
  block it's before has a place.

## Amendment: a loop's `return` MIR joined past a block is a `return` (2026-10-27)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the same jump: the
block's end goes on to that `return;` alone.

React's appendAllChildren (cw1 of the port's gaps) walks a tree with early
`return`s, one inside its inner loop: `while (node.sibling === null) { if
(node.return === null || node.return === workInProgress) { return; } ..
}`. MIR joins the `return`s into one after a block, which the inner loop
leaves by a `break` of the block, and its port printed `bb6: while (..) {
.. if (current !== workInProgress) { while (true) { .. break; } continue
bb6; } return; }`, a label where upstream has none.

**A block MIR made, followed by `return;`, that a loop inside it leaves by
a `break` of it, where a `return` the program wrote is in that loop
(`Hints::returns`), is that `return;` at each of its `break`s**
(`tails::returns_past_blocks`, tidy's `jumps` stage), the block then
spliced in its place: `if (current === workInProgress) { return; } while
(current.sibling == null) { .. return; }`. Not a block the program
labeled, whose `break`s it wrote (this ADR); not a loop's own end, a `for`'s
whose steps run out, which no `return` is in; nor a block no loop leaves,
whose `break`s other passes read as an `if`'s branch, `&&`, `||` or a
guard.

Held by `test/lowering-early-exits.test.ts`, "a loop's early return MIR
joined past a block is a return", run; the mutations
`loop-returns-past-block-kept` and `loop-end-returns-past-block`
(`mir/tidy/jumps/tails.ts`, the latter caught by React's
addValueToProperties, `test/port-performance-track.test.ts`).
