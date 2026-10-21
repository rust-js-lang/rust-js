# 0547. A `copied()` of an iterator with a destructor drops it

Status: Accepted. Extends [0465](0465-dropping-an-adapter-is-closing-it.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): when what an iterator
holds is dropped is Rust's.

## Context

itertools' `chunk_by` gives `Group`s, iterators whose `Drop` tells their
parent they're done; `g.copied().collect_vec()` was refused, "`copied` of
an iterator with a destructor", where `map`, `take` and the other adapters
that hold their source close it to drop it (ADR 0465). And one consumed to
its end by `collect()` dropped nothing, as no JS helper closes what it
finished: Rust drops the adapter, and so its source, as `collect` ends.

## Decision

**`copied()` and `cloned()` hold their source as the others do, closed to
drop it, `$closing`. A `copied()`, or a `cloned()` of what needs no clone,
is its source itself in JS, so consumed by value, `collect()` or `sum()`,
it's closed as the method ends, dropping what it holds**, as an iterator of
the crate's with a destructor is (ADR 0464).

- `is_its_source` is the one answer to which adapters are their source in
  JS: what makes one, and what drops one consumed.

## Why

`consumed_adapter_drops` collects a `copied()` and sums one of the
crate's iterator with a destructor, and takes one item of a `cloned()`,
each dropped where Rust drops it, against native Rust.

## Consequences

- itertools' `chunk_by` groups compile.
- A `map(f)` of one consumed to its end still drops nothing: its JS helper,
  finished, ignores `return()`. It needs an explicit map, which the
  iterators draft's analysis would choose.
- Mutations leave them unclosed, hold no source, drop nothing consumed,
  and map a clone of what needs none.
