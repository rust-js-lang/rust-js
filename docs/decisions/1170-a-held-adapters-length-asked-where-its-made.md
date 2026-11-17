# 1170. A held adapter's length is asked where it's made

Status: Accepted (2026-11-17, the lead's decision; #1795, #1744).

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behaviour is kept. In a crate that reads a `Vec`'s capacity, std
grows a `Vec` by its iterator's hint, so the hint must be std's. JS's own
adapters know none.

## Context

An adapter made in place where a `Vec` is grown by it is made knowing the
length std's growth asks, `$mapEnds($iter(v), f)`. `grown_by_hint` does
this through `as_exact_size`, or `parts_knowing_lengths` for a chain's or
a zip's parts. It reads the adapter's form. A variable holding one shows
no form:

```rust
let d = v.iter().map(|x| x * 2);
let w: Vec<u32> = d.inspect(|_| ()).collect();
w.capacity()
```

`d` was made JS's own map, `v.values().map(f)`, and the growth refused it:
"as an `ExactSizeIterator`, which rust-js can't count" (#1795). A held
chain of an `inspect` and an array's items was refused the same way, by
its parts' hints (#1744).

Making every such variable std's struct, as a held flatten already is
(#1528), compiled both. But it also turned held adapters whose in-place
forms already count, `$skipping`, `.filter` and the rest, into std's
structs. That costs readable output for no change in behaviour.

## Decision

The maker makes the held value knowing what a later use asks, as
`back_stepped.rs` already makes one knowing its back (ADR 0545, #814,
#851). One walk answers both questions about a value, by its maker's MIR
identity:

- `BackStepped::asked_of(owner, local) -> Asked { back, len }`, read by
  lowering as `FnCx::asked_of`.
  - `back` is the `SteppedLocal` it was.
  - `len` is a `LengthAsked`:
    - `Counted`: the temporaries and enumeration sources counted before,
      which their makers make knowing it (#1317, #1354);
    - `Held`: a variable whose length, or whose parts' lengths, the
      growth of a `Vec` by what it's made into asks.
- `local_stepped_from_its_back` and `made_counted_from_its_back` are
  projections of `asked_of`.
- The walk's growth branch asks `lengths_asked(grown)`: of an
  `ExactSizeIterator`, itself and what it passes its length on from
  (`length_kept_source`); of a chain or a zip whose hint is its parts', or
  a copy of one, each part's.
- A variable written with a value made in place, asked `Held` and not yet
  knowing its length, is made so where it's written (`written_as_asked`).
  The making is `made_as_growth_asks`, the owner of what the growth asks,
  which `grown_by_hint` also uses. Its result is marked
  `IterKnowsLength`, a mark the variable keeps. The growth's read takes
  the marked value as it is.
- A value its maker made to step from its back is left as that step made
  it.
- The held flip (`made_knowing`, `decide_held`) reads `Counted` alone. A
  `Held` ask changes a value's form, never whether it's held.

## Proof

- Corpus `held_adapters_whose_length_a_capacity_asks` holds:
  - both issues' programs;
  - a held map that a chain of an `inspect` of it grows a `Vec`;
  - a held map stepped from its back and then grown by;
  - one of items with a destructor;
  - one whose length nothing asks, still JS's own map.
- It matches native in the plain build, with std's iterator structs, and
  through every metamorphic rewrite. On main it was refused.
- The whole corpus passes unchanged, including:
  - `capacity_of_an_enumerated_chars_and_a_slice_split`;
  - `capacity_of_what_std_hints_less`;
  - `counted_adapters_where_capacity_is_read`;
  - the four cases that changed while the held flip read `Held` too.
- Mutations, each caught:
  - `held-length-unasked`
  - `held-length-source-unasked`
  - `held-length-parts-unasked`
  - `held-length-flips-held` (snapshots compared)
  - `held-length-unmade`
  - `held-length-remade-back`
  - `held-chain-parts-unread`

## Consequences

A held adapter whose length a capacity read asks compiles, as one made in
place does. Its readable form keeps being JS's own where nothing asks its
length. The ADR 0908 note about #1744's held form is superseded.
