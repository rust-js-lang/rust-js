# Draft decisions

A draft is a decision proposed, not made: a design for a gap rust-js
refuses today. Each names the ADRs it would extend or amend, the case it
falls in under [0262](../0262-when-rust-and-js-disagree.md), and what would
hold it: the corpus case, the mutation, the test. Proposed examples and
acceptance criteria are not evidence of implemented support.

A draft has no number, so [the architecture test](../../../test/architecture.test.ts)
doesn't read it as a decision, and the index in [docs/README.md](../../README.md)
doesn't list it. To accept one: resolve its open gates, implement it with
a failing test first, give it the next number, move it up a folder, set
its status, and add it to the index. Amend the decisions it changes and
update the semantics and limitations only when the behavior changes.
Keep one account of a decision: remove a draft when an accepted ADR
replaces it.

Estimated difficulty, hardest first; this is not implementation order:

1. [A shipped crate preserves its Rust API's contract](shipped-crates.md):
   separate qualification for [memchr](shipped-memchr.md),
   [regex](shipped-regex.md), and [url](shipped-url.md).
2. [An iterator that's cloned, or holds items with a destructor, is rust-js's own](cloned-and-buffering-iterators.md).
3. [`catch_unwind` catches a Rust panic](catch-unwind.md):
   panic identity, typed payload ownership, and an unresolved abort gate.
4. [Libraries share one counted `Rc` representation](counted-rcs-across-crates.md).

Suggested order: the library representation contract, iterators,
panic support once its abort gate is resolved, then shipped crates one at
a time. A std trait's associated-type drops are already accepted in
[0497](../0497-a-std-traits-dictionary-drops-its-associated-type.md); its
former draft is no longer a separate proposal. `xor` and `zip`
drops are accepted in
[0498](../0498-xor-and-zip-drop-what-they-give-up.md), and `xor`'s cleanup
through a panic in
[0502](../0502-xor-drops-its-second-value-through-a-panic.md).
