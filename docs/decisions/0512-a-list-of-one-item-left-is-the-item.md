# 0512. A list of one item left is the item

Status: Accepted. Extends [0364](0364-mir-lowering.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)): the program can't
tell; the output reads as written by hand.

## Context

MIR holds a list, `println!`'s arguments `(&ids(a), &ids(b))`, while it
takes its items out one by one. Where it had to be written before all
were taken, the drop of what the first borrows coming between, it was
written with a hole for each item taken: `const tmp$1 = [undefined,
ids(b)]`, read as `tmp$1[1]`. No one writes that.

## Decision

**A list with holes whose one item left is all that's read of it, by its
index, is that item: `const tmp$1 = ids(b)`, read as `tmp$1`.** One tidy
pass, `holed_lists`, last of the body's: a list declared or assigned once,
each other mention of it a read of that item.

## Why

- **It's what a person writes.**
- **It's safe**: nothing else reads the list, so nothing can tell it's
  gone; one read whole, or of another index, or written, keeps it.

## Consequences

- Twelve corpus cases lose a hole; a tuple of a unit and a value read only
  by the value, `[true, undefined]`'s `value[0]`, is the value too.
- Mutations keep the list, and take one read whole for its item.
