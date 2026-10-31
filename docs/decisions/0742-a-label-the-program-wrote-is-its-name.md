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

## Amendment: the returns a block's branches wrote, MIR joined past it, are each `return` (2026-10-29)

React DOM's attemptExplicitHydrationTarget (rd100) returns in each branch
of an `else if` chain, `if tag == SUSPENSE { .. if let Some(i) = .. { ..;
return; } } else if tag == ACTIVITY { .. return; } else if tag == HOST_ROOT
&& .. { ..; return; }`, then sets `blockedOn = null`; its
dispatchEventForPluginEventSystem (rd98) returns from two loops inside
`mainLoop`. MIR joins those `return`s into one `return;` of no place after
a block of its own, each branch's `break` of the block, and no loop is
left with one of them, so the rule above didn't take them: the port printed
`bb41: { .. break bb41; .. } return;`, `bb110:` and `bb134:`, labels the
program has none of. So did the playground's two guards one after another
(`wasm/web/rust/components/app.rs`'s `new_file`): `} else if
(!Project.has(..)) { .. } else { .. }` where it wrote `if
project.has(&path) { ..; return; }`.

**A block MIR made, followed by a `return;` of no place, that holds as
many `return`s the program wrote (`Hints::returns`, within the spans of
the block's statements) as it has `break`s, is that `return;` at each
`break` too, each given the place of the written `return` that's its way
out** (`tails::joined_returns`, `tails::returned_as_written`): the first
written after what runs before the `break`. Then each is a `return` the
program wrote to the passes after, which keep it where it is, as they keep
any written `return` (`tail_returns`, `written_thens`). Not a block more of
whose `break`s go on to the function's end than the program wrote
`return`s in it, `if (a) { .. } else if (b) { ..; return; } else { .. }`
(`while_let_at_end`); nor one a `return;` with a place follows, a last
arm's written `_ => { // .. return; }`, which the block's ways go on to
(`match_arm_guard_first`).

Two passes after see that shape too:

- `branch_ends` (`tails::ending`) doesn't make an `else` of `if (c) { s;
  break l; } t` where `t` ends in an `if` each of whose branches returns
  or throws, as it doesn't where `t` ends in a `return` or a `throw`: a
  guard, written so. addValueToProperties' `if (ArrayBuffer.isView(value))
  { ..; break; }` stays the case's guard, what follows it not nested in an
  `else`.
- `let else first`, where the body doesn't go on to its end, takes the
  `return;` MIR made at the end of each way through its last statement
  (`jumps::ended_returns`): a `let … else` taken first leaves its `then`'s
  end the function's, `log.push(`Created ..`); return; }` of nothing.

Case ([0262](0262-when-rust-and-js-disagree.md)): none: each `return;` is
where the block's end would have taken it, the same jump.

The readability ratchet's `return;` share grew (`docs/readability.txt`,
9.9 to 10.7 in 10,000 lines), blessed for this reason: the new case,
`test/corpus/returns_joined_past_blocks.rs`, is ten `return`s its program
wrote and nothing else of the shape, which the shape counts as it counts
MIR's; no other committed JS gained one (the playground's guards are
`.jsx`, which it doesn't read).

Held by `test/corpus/returns_joined_past_blocks.rs` (native-compared: the
rd100 chain, rd98's loops, the playground's guards), the playground's
snapshot, `test/port-performance-track.test.ts`'s addValueToProperties,
and the mutations `returns-past-blocks-joined-unseen`,
`returns-past-blocks-written-return-copied`,
`returns-past-blocks-joined-anywhere`, `returns-past-blocks-unplaced`,
`branch-ends-returning-if-not-an-exit` (`mir/tidy/jumps/tails.ts`) and
`ended-returns-kept` (`mir/tidy/jumps.ts`).

## Amendment: a joined return keeps what's written before it (rd108)

`joined_returns` counted as a block's the `return`s the program wrote
within its statements' places, up to the last one's end. A branch's last
`return;`, after its last statement, `target.blocked_on.set(Some(n)); //
We don't .. return;`, was past that: the block had fewer written returns
than `break`s, so the `return;` that branch got was MIR's, of no place, and
the comment or the blank line written before it was lost, in React DOM's
attemptExplicitHydrationTarget. A block's written returns now reach up to
where the function's next statement is written past its own
(`statement_starts`): each `return;` has its place, and its comment.

```js
} else if (tag === 3) {
  if (n > 5) {
    target.blockedOn = n;
    // We don't currently have a way to increase the priority of
    // a root other than sync.
    return;
  }
}
```

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the same `return`.

The readability ratchet's `return;` share is blessed (10.6 to 10.8 in
10,000 lines): two are the new case's, each a `return;` its program
writes, which no other case changed; the third is
`local_borrow_nested_option`'s, landed beside it, its written `return
None`.

Held by `test/corpus/returns_joined_keep_comments.rs` against native Rust
and the mutation `joined-returns-end-at-last-statement`
(`mir/tidy/jumps/tails.ts`).

## Amendment: a loop's `return` MIR joined past a block is `joined_returns`' (2026-10-30)

`returns_past_blocks` took a block whose `break` is in a loop a written
`return` is in (`left_by_a_return`) for one of returns, besides those
`joined_returns` finds MIR joined there. Since `joined_returns` reads each
`return` the program wrote that MIR joined past the block, a loop's among
them (the amendment above), it found appendAllChildren's too, and no
block reached the pass that only the loop's test held: forcing that test
true and false, the mutations `loop-end-returns-past-block` and
`loop-returns-past-block-kept`, changed nothing in the corpus, the
lowering suites or React's port. The test and both mutations are gone;
`joined_returns` is the rule's one owner. "A loop's early return MIR
joined past a block is a return" and addValueToProperties pass as before.

## Amendment: a joined `return` is the one written after what runs just before it (2026-10-30)

`returned_as_written` placed each `break` of a block MIR joined the
program's `return`s past at the first written `return` after the furthest
written statement its walk had passed, an `if`'s branch among them. MIR
doesn't keep what's written in order: at a function's end, it puts a
`match`'s `_` arm, which goes on to the function's end, before an arm it
tests apart, `if (tag !== 3 && tag !== 27) { log.push(3); return; }
log.push(2); break bb17;`. The walk was then past the `_` arm's
`log.push(3)`, after every written `return`, so the second arm's `break`
had no place, `tail_returns` took it for a `return;` MIR made and dropped
it, and its case printed `break;` out of the `switch` (#435).

A `break`'s `return` is the first written after what runs just before it:
the statement before it in its list, where that is written, whatever the
walk passed before; and after an `if` whose branch leaves, which goes on
to the `break` by its test alone, that test. The two together place each
of a function's returns, or else none, as before: with the first alone,
React's recordLegacyContextWarning, `let Some(root) = root else {
log.push(0); return; }; if fiber.tag == 3 { return; } ..`, MIR's `if
(root != null) { if (fiber.tag !== 3) { ..; return; } break bb; }
log.push(0); break bb;`, placed the `else`'s and not the `if`'s, and
printed `if (fiber.tag !== 3) { .. }` for the guard; with the second
alone, the other way round.

```js
switch (fiber.parent.tag) {
  case 5:
  case 13:
    log.push(1);
    return;
  case 3:
  case 27:
    log.push(2);
    return;
  default:
    log.push(3);
}
```

`let_else_guard_released`, whose `let … else`s' `else`s MIR puts after
the rest, reads as written too: `if (value.value == null) { ..; return; }
console.log(..)` for the second, not `if (value.value != null) {
console.log(..); } else { .. }`, and the first's `else { $unborrow(borrow);
return; }` with the `return;` its program writes.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the same jump: the
`switch` and the `if`s end the function either way.

The readability ratchet's `return;` share is blessed (10.3 to 10.5 in
10,000 lines): two are `pop_host`'s, two `let_else_guard_released`'s,
each a `return;` its program writes.

Held by `test/lowering-switches.test.ts`' "each arm's written return at a
function's end is its case's return", `test/lowering-early-exits.test.ts`'
"a unit function's early returns joined to its end are each a guard
first", `test/corpus/returns_joined_past_blocks.rs`' `pop_host` and
`test/corpus/let_else_guard_released.rs` against native Rust, and the
mutations `returns-past-blocks-placed-past-earlier` and
`returns-past-blocks-placed-past-branch` (`mir/tidy/jumps/tails.ts`).

## Amendment: a test that says no place, and a block left by its own `break`s (2026-10-30, #437, #438)

Compiling React's port (react-in-rust) with the amendment above changed
six of its modules. Three changes were for the better and three were not:
- recordLegacyContextWarning's `if (didWarnAboutLegacyContext.has(..)) {
  return; }` was lost to `if (!didWarnAboutLegacyContext.has(..)) { rest
  }`.
- ScrollEndEventPlugin's `else { // .. return; }` lost its comment.
- diffHydratedStyles' two guards were merged into `if (a === b || (c &&
  d)) { return; }`, losing the second's comment.

Two refinements of "what runs just before the `break`" fix them:

- **An `if` whose test says no place of its own** (a call's value, `if
  DID_WARN.with(|d| d.has(..)) { return; }`, `if
  did_warn.contains(&fiber.tag) { return; }`) is placed by where the `if`
  is written, its condition's place. Its `break` is the `return` after
  that, not one after its branch (#438, the same shape outside the port).
- **A block that its own `break`s leave** goes on to what follows from
  each of them as well as from its end, so not from one place: from the
  furthest point written in it, as before #435. diffHydratedStyles' `bb50:
  { if (a !== b) { if (c) { if (d) { break bb50; } } ..; return; } } break
  bb;` goes on to the `break` from both of its guards, so that `break` is
  neither's `return`, and the inner block places each guard's own.

With both, React's port differs from its committed JS in four modules,
each closer to upstream or to its Rust:
- diffHydratedStyles: its two guards, as upstream writes them.
- EnterLeaveEventPlugin's `// Nothing pertains to our managed
  components.` before its `return;`.
- SimpleEventPlugin's `if (withBorrow == null) { return; }`, upstream's
  guard, not the rest nested in its negation.
- throwAndUnwindWorkLoop's `if (didFatal) { return; }`, as its Rust
  writes it.

The corpus is as before.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the same jumps.

Held by `test/lowering-early-exits.test.ts`' "a unit function's written
guards joined to its end are each where written", run on each path, and
the mutations `returns-past-blocks-unspanned-test-past-branch` and
`returns-past-blocks-left-block-one-place` (`mir/tidy/jumps/tails.ts`).

## Amendment: a `return` joined past a block inside another is the inner block's (2026-10-31)

React DOM's preload is a let chain's `if` around everything it does, its
body's `if … else if` two early `return`s, each under its comment:

```rust
if let Some(owner_document) = owner_document && !href.is_empty() && !r#as.is_empty() {
    ..
            if r#as == "style" && .. {
                // We already have a stylesheet for this key. We don't need to preload it.
                return;
            } else if r#as == "script" && .. {
                // We already have a script for this key. We don't need to preload it.
                return;
            }
    ..
}
```

MIR joins the chain's failing links and the two `return`s into the one
return, so a block it made around the chain, `bb53: { if (owner != null)
{ .. break bb53; } break bb53; } return;`, has two `break`s and two
written `return`s in it, and `joined_returns` took them as the block's:
the JS was `if (!href) { // We already have a stylesheet .. return; } if
(!as) { return; } ..`, each comment on a link of the chain, the chain
gone to guards. The two `return`s are a block's inside it, `bb55: { ..
break bb55; .. break bb55; } return;`, which MIR joined them past.

So a `return` written in a block inside the one asked about, which a
`return;` MIR joined follows too, is that block's, not one of the outer
block's joined ones. The chain's `break`s then have no written `return`
and stay the chain's: `if (owner != null && href && as) { .. }`, each
comment on its own `return`, as upstream writes it.

Alternatives: running `returns past blocks` innermost first, so an inner
block took its `return`s before the outer counted them, says the same,
but changes where `returned_as_written` places other blocks' `return`s,
which reads statements outermost first: diffHydratedStyles' two guards
and DOMPluginEventSystem's `while` came back nested and labeled.

The corpus is as before; of React's port, only preload changes.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the same jumps.

Held by the corpus case `let_chain_around_early_returns` and the
mutation `joined-returns-inner-taken` (`mir/tidy/jumps/tails.ts`).
