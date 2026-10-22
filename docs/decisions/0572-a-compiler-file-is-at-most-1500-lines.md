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

- `test/architecture.test.ts`, "a compiler file is at most the cap, one
  listed over it only shrinking".
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
