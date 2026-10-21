# A library rust-js can't compile whole is compiled item by item

Status: Draft; awaiting the user's decision. Would amend
[0415](../0415-a-crate-of-crates-io-in-an-app.md), whose crate is
compiled whole or is its types alone.

Case: D ([0262](../0262-when-rust-and-js-disagree.md)) kept: what can't
be kept is a compile error. N for where the error is reported: at the
consumer's call, not at the library.

## Context

Under ADR 0415, a library with one item rust-js refuses is its types
alone, and every call into it is refused. aho-corasick, which regex
needs, compiles but for items regex never calls:

- its stream search, generic over an `R: io::Read`, which ADR 0425
  refuses as JS has no stream;
- `Debug` of memchr's `memmem::Finder`, which a derived `Debug` of its
  prefilter shows, and memchr's qualified API (ADR 0529) doesn't cover;
- an unaligned read and two pointer addresses, addressable memory.

Any one stops the crate, and with it regex. A program that calls
`Regex::new(..).is_match(..)` never reaches any of them.

## Proposal

**A library is compiled item by item. An item rust-js refuses is
written as no JS, and its manifest says it's refused and why. A call to
it, from inside the library or from its consumer, is refused there,
naming the item and the reason. An item that calls a refused item is
refused too, transitively, and says through which.**

- **What's refused is a function, a method or a trait impl's method, never
  a type.** A type's representation is the library's whole decision
  (ADR 0523). A type rust-js can't represent still refuses the crate.
- **A trait impl with a refused method** is a dictionary whose entry for
  that method is missing. A call through the dictionary is refused where
  rustc resolves it to that impl, and where it can't, through a type
  parameter, where the consumer gives the dictionary, naming the method.
  A derived `Debug` that shows a refused field is that impl's `fmt`,
  refused where something formats the type.
- **The manifest** (`library::Item`) gains `refused: Option<String>`, the
  reason. A refused item has no export. The transitive closure is
  computed once, in the library, from its call graph: a consumer only reads it.
- **Nothing is refused at run time.** There's no stub that throws: every
  refusal is a compile error at a call rustc sees. A call reached only
  through a `dyn` of a refused impl method is refused where the `dyn` is
  made, which is the last place rustc sees the impl.

## Open gates

1. Whether "refused where called" may be reached through generic code
   in the library, where the call site that names the concrete type is
   the consumer's: the dictionary route above, proved by a two-crate
   test.
2. Drop glue: a type whose `Drop` impl is refused is refused whole, as
   dropping is everywhere.
3. `docs/crate-corpus.md` gains a column, "compiled, N items refused",
   so a crate that compiles in part isn't counted as compiling.

## What would hold it

- A two-crate test: a library with a refused function, a refused trait
  method and a refused derived `Debug`, and a consumer that calls the
  rest, which runs as native Rust, and diagnostics tests of each refusal
  at the consumer's call.
- regex's probe, once aho-corasick's memory sites are answered or are
  themselves refused items, run against native Rust.
