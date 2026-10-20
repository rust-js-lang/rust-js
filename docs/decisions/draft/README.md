# Draft decisions

A draft is a decision proposed, not made: a design for a gap rust-js
refuses today, written in the form an ADR takes so that accepting it is
renaming it. None is implemented; each names the ADRs it would extend or
amend, the case it falls in under [0262](../0262-when-rust-and-js-disagree.md),
and what would hold it: the corpus case, the mutation, the test.

A draft has no number, so [the architecture test](../../../test/architecture.test.ts)
doesn't read it as a decision, and the index in [docs/README.md](../../README.md)
doesn't list it. To accept one: give it the next number, move it up a
folder, set its status, add it to the index, then make it true, a failing
test first.

Ranked by how hard each is to close, hardest first:

1. [rust-js ships the crates that read bytes at addresses](shipped-crates.md):
   regex, memchr and url, platform-backed, as hashbrown's maps are std's.
2. [An iterator that's cloned, or holds items with a destructor, is rust-js's own](cloned-and-buffering-iterators.md).
3. [A library's manifest says which `Rc`s it counts](counted-rcs-across-crates.md):
   itertools' `tee`.
4. [A std trait's dictionary has its associated types' drops](std-dictionary-associated-type-drops.md):
   `<T as Add>::Output`, the change on disk, finished.
5. [`xor` drops what it gives up](xor-drops.md).
6. [`catch_unwind` is a `try`](catch-unwind.md).
