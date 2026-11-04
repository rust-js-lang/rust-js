# 0979. A dictionary a body writes twice is declared once at its top

Status: Accepted, under the user's goal of 2026-10-26 ("keep going until
all done"). The plan "rust-js: one representation per iterator", toward
its fourth phase, readability parity. Extends
[0098](0098-destructors.md)'s rule that a long drop is written once:
the same of a dictionary. Amended: a generic impl's dictionary, made of what
the function is given, too (#691).

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

## Amendment: a generic impl's dictionary, made of what the function is given (#691)

Case ([0262](0262-when-rust-and-js-disagree.md)): none, no observable
difference. A generic impl's dictionary is its function's call,
`holderTouch(TTouch)`, which `$traitImpl` caches by what it's given: each
call gives the same object, and making it runs nothing of the program's
but its entries' closures. Read once at the body's top, even ahead of a
branch not taken, it gives what each place it's asked gave.

A generic impl's dictionary was written at each place a body asked it:
a copied default calling its own impl through the dictionary's entry
(#690) wrote `holderTouch(TTouch).me(self)`, and so did generic code,
`both<T: Touch>(h: &mut Holder<T>)` calling `h.twice()` twice, and
`main` of a type known whole, `tExt({ next: $rangeNext })` twice. Nobody
writes a lookup again at every call.

- **What's kept.** Besides the literal of a type known whole, a body
  keeps each dictionary made that is a call of one of the program's items,
  a generic impl's function, of a type known whole or not, a copied
  default's of its own impl among them: its `Self` dictionary is the
  impl's, made of what the impl is given.
- **What it may read.** What a body's top can: the runtime's helpers, the
  program's items, what its own functions bind, and what the function is
  given (`given_names`, `traits/given.rs`): its dictionaries, `TTouch`, and
  what a copied default's made of them reads, its const parameters'
  values, facts, caller's location and drops, `dropI`. A closure's top
  reads its function's. A drop the body makes for itself isn't given, so
  a dictionary that reads one stays where it is.
- **Names.** A bound's dictionary's, as a function's parameter is named,
  of its type and its trait: `HolderTouch`, `RangePull`. Two of one name
  are told apart, `HolderTouch` and `HolderTouch$1`, as two literals are.
- **Where.** The body's top, as before, before tidy: a copied default's
  at its own, `const HolderTouch = holderTouch(TTouch);` ahead of
  `HolderTouch.me(self)`; one a closure writes twice at the closure's. One
  written once stays where it's asked. The largest is named first, as
  before, so a literal a call is given, `fuseDebug`'s `{ fmt }` of
  `peekableDebug(fuseDebug({ fmt }), debug)`, that's written nowhere else
  stays in the call's `const`, where it was a `const` of its own when the
  call was written three times.

Twenty-five corpus snapshots change, 162 lines added and 131 removed.
`test/corpus/generic_dictionary_declared_once.rs` runs against native
Rust: a copied default calling its impl's dictionary twice, and behind a
branch not taken; generic code, a closure in it and a branch not taken;
generic code given a drop its impl's dictionary takes, `slotRenew(TDefault,
dropT)`; `main`'s dictionaries of types known whole, one of another. The mutations
are `generic-dictionary-repeats-unnamed` (`traits/dictionaries.ts`),
`given-names-without-evidence` and `given-names-without-drops`
(`traits/given.ts`).
