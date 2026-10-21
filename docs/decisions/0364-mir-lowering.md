# 0364. Bodies are lowered from their MIR

Status: Accepted. MIR is the one lowering of bodies; THIR's is retired.
Amended: a type is an iterator where rustc says its `Item`, a type or
another trait's associated type, smallvec's `A::Item`, not one of std's
iterator traits' still: `implements_iterator`, which `for _ in &mut it`
asks (`test/corpus/iterator_of_projection.rs`). Amended by
[0569](0569-a-temporary-is-named-as-rust-analyzer-suggests.md): a
temporary tidy keeps is named as rust-analyzer suggests, after its call or
its type, before `value`.

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

rust-js lowers each body from its THIR, the tree of what was written. JS
written from a tree reads as the Rust does, but what the tree leaves
implicit, rust-js had to work out case by case, each an ADR: the order
operands run in and when a value is read (a key against its children,
a struct's base after its fields, `unwrap_or_else`'s operand), how long a
temporary lives (a `RefCell`'s guard across JSX, ADR 0328), what a
reborrow or a coercion refers to, what's moved and what's dropped (ADR
0098), and on a panic, what's dropped as it unwinds.

MIR, what rustc builds for borrowck, says each of these: every value in a
temporary of its own, assigned in the order it's made, each borrow, move,
drop and check a statement or a terminator of its own. It's what Miri
runs, the semantics rustc defines.

## Decision

**Correctness is mandatory; readability is secondary.** MIR supplies
execution facts, but using MIR does not itself prove that rust-js's lowering
is correct. Tests and mutations must establish that the generated program
preserves the supported Rust semantics and documented differences.
Necessary temporaries, checks, helpers and cleanup blocks are acceptable;
readability rewrites are made only where they preserve semantics.

**A body is lowered from its MIR, as borrowck reads it, after promotion
and before drops are elaborated. Its THIR is beside it, for its shape:
names, spans, what a std macro made.** The JS is correct as it's made,
and made readable only where that's proved to change nothing:

- **Structure**, by Ramsey's "Beyond Relooper": a reducible graph, as
  Rust's are, is labeled blocks and loops, `break` and `continue`. A
  panic is a JS exception, so unwind edges and cleanup blocks are left
  out, and so are the edges borrowck alone reads.
- **Returns where they're written**: a branch to the block that only
  returns is the `return`; the return place assigned and returned is the
  value returned; a branch that returns or throws is written first, as an
  early exit, and what follows it unnested. A `break` that ends its own
  block, and a label nothing names, go.
- **`?` is what it tried**: `Try::branch` of an `Option` or a `Result`
  makes no `ControlFlow`; its `Continue` is the value inside, its `Break`
  the `None` or the `Err` itself, and `from_residual` THIR's conversion.
- **Folding**: a temporary made once and read once is written where it's
  read, where that changes no order. Those read that do something are
  read in the order they were made; what was made after them, and isn't
  read, moves freely; what's read before them can't change meanwhile.
  Otherwise each is a `const` where it's made.
- **What moves freely**: what does nothing and reads variables nothing
  borrows; and a place shared, or shown by `format_args!`, which borrowck
  keeps unchanged while the reference lives, read where it's used, as
  Rust reads it.
- **One std layer**: a std function's JS is made of its arguments' values
  and types, `std_values`. A handler that needs what THIR shows of an
  argument says so, and of a call from MIR takes the answer that's always
  correct: a `RefCell`'s borrow counted, an index checked.
- **Drops, as rustc elaborates them**: rustc's own dataflow of what's
  initialized says, before each `Drop`, whether what it drops is there,
  isn't, or may be, and a flag of its own, set where rustc's
  `ElaborateDrops` sets it, says which as the body runs. A call that may
  panic while something's there to drop runs in a `try` whose `catch`
  drops what its cleanup blocks drop and throws the panic on: the
  statement that runs it, as it may be written where its value is used,
  so the calls written in one statement share a cleanup, and a flag set
  or a different cleanup runs what's made first.
- **Representations are types'**: a cell of a local or a field (ADRs
  0270, 0288, 0362) stays a cell, made plain only where MIR's own analysis
  of its borrows says nothing else sees it change: a module's own
  thread-local, a cell only its function uses.
- **Integers wrap**: rust-js's session builds MIR with overflow checks
  off, as its semantics are the release profile's (ADR 0011).

## Why

- **Correct by construction**: each order, borrow and drop is what rustc
  says, not what rust-js worked out of a tree.
- **Readable by proof**: every rewrite toward the JS a person writes has
  a reason it changes nothing; where there's none, the JS is plainer, not
  wrong.

## How MIR became the one lowering

MIR became the default once correctness was shown for the supported scope:
the corpus agreed with native Rust under the documented semantics, the
supported rustc tests and mutations passed, react.dev's 823 pages kept
their behavior and public contracts, and diagnostics, interop and
native/browser consistency were checked. THIR's output was a readability
reference, not the authority on behavior.

Readability parity with THIR or the original JavaScript did not block the
switch. Output differences were reviewed for semantic and public-contract
changes; those that only affect readability are tracked as later
improvements, each of which must keep evaluation order, side effects,
copying, borrows, drops and failure behavior.

THIR's lowering of bodies is removed, and `RUST_JS_MIR` with it. Its last
uses each have a MIR lowering:

- **Writers** (ADR 0054): a `Formatter` parameter is the string `f` it
  writes to, a write `f += s`, and `f` what the writer gives back;
  `return s` of each way through that writes once. The `Formatter`'s
  options (ADRs 0058, 0137), its questions (`f.alternate()`, ADR 0143), a
  `Debug` builder's chain (ADR 0136), a writer that may fail (ADR 0187)
  and the `Formatter` given to generic `fmt::Write` code (ADR 0180) are
  lowered as THIR's were, and what THIR refused, a builder kept in a
  variable, is refused.
- **Initializers** (ADR 0096): a static's or a constant's lowered as code,
  a `const { .. }` block's, and a generic impl's constant (ADR 0176), from
  their MIR. Their checked arithmetic, which rustc evaluates, is the
  operation; a promoted value rustc can't say as JS, `&[("double",
  double)]`, is its own MIR lowered as code.
- **A trait's default copied into an impl** (ADR 0176): the default's MIR,
  as the trait has it, lowered in the impl's dictionary, its types asked in
  the impl's terms, an object `Self` given its impl's method as the object
  and a number's as its box.

A body whose MIR rustc evaluated before rust-js read it, a `const fn` a
pattern's constant calls, is read from the MIR rustc keeps for evaluation;
constants and `const fn`s are read first. What THIR refused only for its
own reasons and MIR lowers as Rust runs it is a corpus case that runs
against native Rust.

THIR is still read beside each body's MIR: for its shape, the names,
spans and what a std macro made, by which tidy writes the JS as it was
written, and for the facts its analyses give. Removing the lowering of
bodies didn't remove that.

## Amended by

- [0372](0372-a-check-that-drops.md): A check whose panic drops is written where it's made

## Amendment: what several cases test is made once

A `match` of a call's value, `match note(15, x) { 1 => .., 2 => .., _
=> .. }`, called it once per arm it tested: MIR folds the call into its
`switchInt`'s one use, and the switch is a test of each case. A
generated program found it, seed 50147 (2026-10-14). What more than one
case tests, and does something as it's made, is held first, `const n =
note(15, x);`, as Rust makes it once; one that only reads, `i % 3`, is
read by each test, as before. `test/corpus/match_call_once.rs` runs it
against native Rust; a mutation calls it again.

## Amendment: nested loops read as written

heck's word splitter, a `while let` with a `peek()` inside a `for`,
read as no one writes it: `let nextMode;` declared before the outer loop,
the inner loop in a labeled block, `bb103: { .. break bb103; return f; }
continue bb3;`, and a `return` after the `break` that nothing reached.
Each rule below changes no meaning:

- **A `let` sinks through a loop or a labeled block** that is all that
  names it, whose body assigns it before anything reads it: the variable
  Rust declares inside, `const nextMode = ..` where it's made.
- **A labeled block followed by a labeled jump**, `l: { .. break l; .. }
  continue m;`: each `break l` is that jump, which goes on from there
  anyway, and the label nothing names then goes.
- **A loop that ends the body of loop `l`**, one the program names no
  label: each `continue l` of its own, not a loop's inside it, is its
  `break`, as Rust's `break` there is, and `l`'s label goes; then an
  early exit, `if (!value) { break; }`, as before. A `continue 'rows` the
  program wrote stays. This runs once what's after each loop is placed,
  which reads where its `continue l` goes.
- **What those jumps leave past them goes**, as it did before them.

`test/corpus/nested_loop_jumps.rs` and heck's `transform`, in
`writer_callables.rs`, hold each, against native Rust; a mutation of each
is caught.

