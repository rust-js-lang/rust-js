# 0572. A compiler file is at most 1,500 lines

Status: Accepted. Extends [0084](0084-owned-phases-and-host-boundaries.md),
whose owned phases this holds to a size each can be read at, and
[0565](0565-readability-is-a-gate.md), whose ratchet this is of the
compiler's own code.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. Nothing a program
can observe changes; this decides what the tests hold.

## Context

AGENTS.md asks that each piece be understood on its own: a module owns one
responsibility, so it can be read, changed and tested without holding the
rest in your head. Nothing held how big a module may grow, and some grew
past what can be read at once. `bun run file-sizes` counted the compiler,
`src/` and `build.rs`, at 464 files and 96,250 lines: the median Rust file
is 289 lines and nine in ten are under 1,333, but `recognition.rs` is
7,535 and `mir.rs` 7,362, one `impl FnCx` of 6,000 lines in it. Those files
hold several concepts each, `recognition.rs` std's names, serde's, JSON's
and some sixty predicates, and an agent reading one in pieces is the
reader most likely to miss the one place that answers its question and
write a second.

## Decision

**A compiler file is at most 1,500 lines, and one over it when the cap
came may only shrink.** `scripts/file-sizes.ts` owns the cap, `LARGE`,
what it's held over, `COMPILER`, and how a file's lines are counted;
`bun run file-sizes` lists them, and `test/architecture.test.ts` holds
them:

- A file not listed over the cap fails the suite.
- The thirteen files over it now are listed with their lines; one that
  grows past its number fails.
- One brought under the cap, or split away or moved, must leave the list,
  so the list only gets shorter.

Every line counts, comments too, and every source file under `src/`,
tracked or new, the runtime's helpers with the Rust.

## Why

- **1,500 catches the outliers alone.** It's past nine Rust files in ten,
  so the files it flags are the ones holding several concepts, not a
  focused module of 1,000 lines split to meet a number. 1,000, the figure
  most often given, flags 24 today, `bindings.rs` and `lower.rs` among
  them, each one concept; 2,000 lets `display.rs`, at 1,996, keep growing.
- **A ratchet, not a cleanup first.** Splitting thirteen files before the
  cap holds would leave them free to grow meanwhile; listed, none can, and
  each split lands on its own, a move the corpus proves changed nothing.
- **One place counts.** The test, the report and the architecture
  report's largest modules ask `fileSizes`, so the number the test fails
  on is the one the report shows.

## Alternatives

- **No cap, a report read by hand**: what was, and the files grew.
  Rejected.
- **A cap of 1,000 or 2,000**: above. Rejected for now; 1,000 is worth
  revisiting once the large splits are done.
- **Count code alone, not comments**: a file is read whole, and this
  repository's comments say why; a second count is a second way.
  Rejected.
- **Tighten a listed file's number each time it shrinks**: would hold
  what a split gains, at the cost of an edit to the test in every change
  that shortens one. Not now; the cap still bounds each file.

## Consequences

- A change that would take a file past the cap splits it by concept in
  the same change, or first.
- `recognition.rs`, `mir.rs` and `traits.rs` are the first to split, each
  a pure move in its own commit.
- A file split below the cap is taken off the list in the same commit.

## What holds it

- `test/architecture.test.ts`, "a compiler file is at most the cap" (the
  list of files over it, and its ratchet, went with the last of them;
  see the last amendment).
- `scripts/file-sizes.ts`, the one place that counts.

## Amendment: recognition, split by concept (2026-10-22)

`recognition.rs`, 7,535 lines, is the first split, a move of its items and
nothing else: the corpus, the snapshots and the moved mutations hold what
it does. It's one owner still, a folder whose root keeps `Recognition` and
`Std`, the question and the answer, and re-exports its parts, so every
caller asks it as it did:

| Part | What it answers |
|---|---|
| `calls.rs` | Which `Std` a call is: `classify`, of a function, a trait method, an index or a conversion |
| `inherent.rs` | Which `Std` a std type's own method is |
| `types.rs` | What a std type is, its facts, and what it holds |
| `iterators.rs` | An iterator's source, its adapters, and its traits' methods |
| `traits.rs` | Whose impl a call reaches, what a bound says, a trait's operators and conversions |
| `std_items.rs` | Where std defines each item, which rust-js compiles |
| `fmt.rs` | `fmt`'s traits, a `Formatter` and its builders, and the writers |
| `wrappers.rs` | `Rc`, cells and guards, pins and futures, and a borrow of one |
| `serde.rs`, `json.rs` | serde's traits and derives; rust-js's JSON types |

Each part is `use super::*`, as `mir/`'s are; an item one part asks of
another is `pub(super)`, and one the rest of `lower` asks is
`pub(in crate::lower)`, as it was `pub(super)` before. Each part's
mutations are its own list. The largest part is 1,196 lines, and
`recognition.rs` leaves the cap's list.

## Amendment: mir, split by what it lowers (2026-10-22)

`mir.rs`, 7,362 lines, one `impl FnCx` of 6,000 in it, is split the same
way, a move of its methods and nothing else. Its root keeps the body's
state, `State`, `Found` and `Locals`, the signature and `mir_function`;
the parts are `structure.rs` (blocks, terminators, a switch's tests),
`temporaries.rs`, `locals.rs`, `patterns.rs`, `closures.rs`, `assign.rs`,
`places.rs`, `operands.rs`, `rvalues.rs`, `dispatch.rs` (`mir_call` and
`mir_std_call`), `jsx_props.rs`, `entries.rs` and `queries.rs`, the
largest 933 lines. The architecture test lists each part's fields of
`State`, as it lists `drops.rs`'s, and the parts that read `locals.mir`.
`mir_call` is held at 107 lines, two more than it was: a path the move
spelled from the crate, `crate::lower::`, is wrapped by rustfmt.

## Amendment: traits, split by what it builds (2026-10-22)

`traits.rs`, 4,128 lines, keeps bounds and evidence, `Given`, and the
validation of what the crate implements; its parts are `routes.rs`
(where a dictionary is), `dictionaries.rs`, `impls.rs` (an impl's
dictionary lowered, and a copied default), `dyns.rs`, `calls.rs`,
`given.rs`, `drop_params.rs` and `words.rs`, the largest 840 lines. The
parts that read `self.given` are its owners, and the rule that keeps
method-name matches out of `traits.rs` holds over its parts too.

## Amendment: the JS tree, its walks apart (2026-10-22)

`js.rs`, 2,443 lines, keeps the tree's types and its tests; what makes an
expression is `js/make.rs`, and what walks one, `js/expr_walk.rs`, and a
statement or a block, `js/stmt_walk.rs`, each re-exported, so `js::` is
asked as it was. `js/` is owned output's, as `js.rs` is.

## Amendment: iterators, by what's done to a chain (2026-10-22)

`iterators.rs`, 2,300 lines, keeps what a value is as an iterator, held
or given as it is, and the JS forms of one; its parts are `stepping.rs`,
`knowing.rs` (its back, length and size), `dictionaries.rs`, `ends.rs`,
`eager.rs` and `in_place.rs`, the largest 526 lines. `Iterator.from`,
`$iterator`, `$iter` and `$lent` are written by the parts that write them,
listed as their owners beside `iterators.rs`.

## Amendment: display, `{:?}` and what's written apart (2026-10-22)

`display.rs`, 1,996 lines, keeps `Writing`, `Pretty` and a `fmt::Result`'s
answers; `display/written.rs` is what a `Display` or a `Debug` builder
writes, and `display/debug.rs` `{:?}` of a value. Both read
`self.writing`, and are listed as its owners beside `display.rs`.

## Amendment: tidy's jumps, by the jump (2026-10-22)

`mir/tidy/jumps.rs`, 1,724 lines, keeps early `break`s and `return`s;
`continues.rs`, `tried.rs`, `tails.rs` and `labels.rs` are the rest,
under `mir/tidy/jumps/`, the largest 442 lines.

## Amendment: MIR's iterators, by what's done to a chain (2026-10-22)

`mir/iter.rs`, 1,687 lines, keeps `mir_iter_call` and `mir_iter_method`,
the dispatchers the architecture test holds; `collect.rs`, `backs.rs`,
`closing.rs` and `sources.rs`, under `mir/iter/`, are the rest.

## Amendment: oxc's tree, statements and expressions apart (2026-10-22)

`to_oxc.rs`, 1,657 lines, keeps `emit`, the header and the comments;
`to_oxc/statements.rs` and `to_oxc/expressions.rs` convert the rest.
`to_oxc/` is printing's, so oxc stays behind the same adapters.

## Amendment: combinators, the dispatcher apart (2026-10-22)

`combinators.rs`, 1,631 lines, keeps `Comb`, `IterComb` and `comb_values`,
the dispatcher the architecture test holds; `combinators/values.rs`,
`adapters.rs` and `callbacks.rs` are the rest.

## Amendment: a number's operators and generic calls apart (2026-10-22)

`std_types/number.rs`, 1,608 lines, keeps `NumOp` and a number's methods;
`std_types/number/operators.rs` and `generic.rs` are its operators and
its calls of a generic number's type.

## Amendment: drops, by what's dropped (2026-10-22)

`drops.rs`, 1,564 lines, keeps `DropState`, a body's drop queries and the
checks of what it's given; `drops/parts.rs`, `scopes.rs`, `functions.rs`
and `uses.rs` are the rest, beside `types.rs` and `facts.rs`, each listed
as an owner of `self.drop_state`. A `DropUses` is written beside its
fields, in `uses.rs`, with the two calls that write them.

## Amendment: serde's JSON reading, a helper for each reader (2026-10-22)

`runtime/from_json.js`, 2,009 lines, is six helpers, in the order they
extend `$JsonDecoder`: `json_decoder.js`, `json_reader.js`,
`json_content.js`, `json_flat.js`, `json_value_reader.js` and
`from_json.js`. `Helper` lists them in that order, so `@rust-js/runtime`
is the same module, byte for byte.

With it no compiler file is over the cap, and the list of those that were
is gone, and its ratchet with it: the test holds every file at 1,500
lines, and one that would grow past them is split first.

## Amendment: a test file too (2026-10-22)

A test file had no cap, and `test/lowering.test.ts` grew to 13,712 lines,
343 tests of every concept the compiler has, in the order they came:
where a test of one is, and whether one is there already, is found only
by searching it all, and a mutation's tests name a file of everything.
Decided with the user: **a test file is at most the cap too**, the tests'
own code, `TESTS`, `test/*.ts`, not a snapshot or a corpus case the
compiler wrote or reads. The same ratchet as the compiler's first: the
three over it, `lowering.test.ts`, `jsx.test.ts` at 6,954 and
`bindings.test.ts` at 2,378, are listed with their lines, may only
shrink, and leave the list once split under the cap, each split by
concept, a move of its tests and the mutations that name them, in a
commit of its own. `test/architecture.test.ts`, "a test file is at most
the cap, or shrinks", holds it.
