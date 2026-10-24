# 0742. A block's label the program wrote is its name in JS

Status: Accepted. Keeps [0158](0158-labeled-blocks.md), a labeled block
is JS's labeled block, under MIR's lowering. Amended by
[0790](0790-a-break-the-program-wrote-is-where-it-is-written.md): the
block is named first, its `break`s are where they're written, never an
`else` or a `return` tidy makes of them, and a block that ends a case is
the case's `break;`s, no label.

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
