# 0753. A branch of only comments keeps them

Status: Accepted. Extends [0444](0444-a-comment-is-where-its-code-is.md):
a comment with no statement after it in its block is printed there.
Supersedes its consequence that an `else` of only comments is no `else`.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: comments, which
nothing runs.

## Context

React's `addObjectDiffToProperties` (shared/ReactPerformanceTrackProperties.js)
says why a branch does nothing:

```js
if (indent >= 3) {
  // Just fallthrough to print the two values if we're deep.
  // This will skip nested properties of the objects.
} else if (/* .. */) {
```

A comment is its next statement's (ADR 0444), and this branch has none.
MIR has no statement of it either, only the jump past the chain, so tidy
saw `if (indent >= 3) { break bb77; }`, took it for an `if` of nothing and
turned the chain, `if (indent < 3) { if (..) { .. } }`, the comments gone.
Decided with the user (2026-10-24): comments aren't nothing.

## Decision

- **A block of only comments is a statement, `StmtKind::Comments`,
  where it ends**: lowering makes one of the `()` MIR assigns of the block,
  whose span is the block's (`sources::comments_end` says whether it holds
  only comments, and where its `}` is). Nothing runs of it; tidy keeps it,
  so the branch is one of something, as it's written.
- **The printer writes its comments there**: they're attached to it, and,
  no node starting where it is, given to oxc as comments whose node is
  gone, which it prints before what follows, or before the block's `}`,
  to which the block's span reaches. `if (indent >= 3) { // .. } else if`.
- A branch with no comment, `else {}`, is nothing, and is still left out.

## Why

- **It's upstream's JS**, the chain as it's written and the comments that
  say why its first branch is empty.
- **A comment is where its code is (ADR 0444)**: here, the end of a block.

## Alternatives

- **The comments before the `if`**: they'd say why of what isn't there.
- **An empty statement, `;`, holding them**: JS no person writes.

## Consequences

- `test/lowering-comments.test.ts` holds a branch's and an `else`'s; the
  port's `addObjectDiffToProperties` has upstream's chain.
