# 0979. A dictionary a body writes twice is declared once at its top

Status: Accepted, under the user's goal of 2026-10-26 ("keep going until
all done"). The plan "rust-js: one representation per iterator", toward
its fourth phase, readability parity. Extends
[0098](0098-destructors.md)'s rule that a long drop is written once:
the same of a dictionary.

Case ([0262](0262-when-rust-and-js-disagree.md)): none, no observable
difference. A dictionary of a type known whole is pure: objects and
functions of helpers and the program's items. Made once at a body's top,
it gives what each place made it gives.

## Context

With std's structs compiled, each compiled method is given its sources'
dictionaries, and each dictionary's entries give their sources' in turn.
The same literal was written again at every place it was asked: `rev()`
of a chain wrote `Chain`'s `DoubleEndedIterator`, four functions each
given four dictionaries, once for `$knowing` and once for `rev`'s bound,
and the same of each source. With std's structs alone `rev_chain` was
1,933 lines, 64 in the normal build, and the corpus was 22.3% longer
than the normal build's. The normal build repeated small ones too:
`{ len: (iterator) => $exactLen(countdownIterator_sizeHint(iterator)) }`
three times in a statement.

## Decision

A body lowered from its MIR, a function's or a closure's, keeps each
dictionary it makes that is of a type known whole and holds a function.
When the body is done, before tidy, each kept dictionary whose literal the
body writes twice or more is:

- declared once at the body's top, `const chainDoubleEndedIterator = {
  .. }`;
- named everywhere it was written, the largest first, so one inside
  another is named in its `const` too.

One written once stays where it's asked. A consumer that reads its
dictionary's literal, a function pointer that takes its `fmt` out of it,
still sees the literal: what's declared is decided only when the body is
done.

- **What it may read.** It reads only what a body's top can: the
  runtime's helpers, the program's items (`Known::Item`), and what its own
  functions bind. One that reads a variable of the body stays where it is.
- **Names.** Its type's and its trait's, `chainDoubleEndedIterator`,
  `u32Clone`. `unit`, `tuple` and `items` stand for types with no name of
  their own. The trait's alone is used where one literal serves several
  types: `clone`, the identity clone of many `Copy` types.
- `{ next: $next }`, of helpers alone, reads as well inline and stays.

## Consequences

`rev_chain` with std's structs is 526 lines. The corpus with std's
structs is 13.0% longer than the normal build, from 22.3%. 114 normal
snapshots change, by 968 lines added and 1,773 removed. The readability
ratchet's shares of `while (true)`, `Iterator.from` and renamed `x$1`
grow by a little: their counts are the same or nearly (two dictionaries
of one trait's name are `intoIterator` and `intoIterator$1`) in fewer
lines.

The mutations are `dictionary-declared-once-used`,
`dictionary-repeats-unnamed` and `dictionary-of-steps-declared`.
