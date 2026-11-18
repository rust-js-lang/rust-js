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
  a copy of one, each part's. It marks a variable `Held` by its MIR
  identity, as the rest of the walk does: walking back from what the
  growth's call is given, through moves, borrows and the calls that made
  each value of an asked type. Another variable of the same type stays as
  it was. (Amended 2026-11-17: the first form marked every variable of an
  asked type in the body, the lead's review.)
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
  - one whose length nothing asks, still JS's own map;
  - of two variables of one adapter type, only the one the growth is made
    of, made knowing, the other JS's own map.
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
  - `held-length-by-type` (snapshots compared)

## Consequences

A held adapter whose length a capacity read asks compiles, as one made in
place does. The mark a variable keeps of JS's own iterator form,
`Known::IterHintless` (#1792), no longer guards that capacity, which the
maker now gives. It still answers #1803's other question for a variable:
whether its form has a fold, so a `Vec` extended by an adapter of it is
`$extend(w, d.map(g))`, not given the `true` that changes nothing. Corpus
`extend_by_an_adapter_of_a_held_js_iterator` holds that against native,
with snapshots compared, and mutation `hintless-form-forgotten-by-its-variable`
tests it there. Its readable form keeps being JS's own where nothing asks its
length. The ADR 0908 note about #1744's held form is superseded.

## Amendment (2026-11-18, #2041): a temporary's length asked too, and a flushed one made so

`vec![1, 2].into_iter().inspect(f).chain(vec![9]).collect()` was refused
in a crate that reads a capacity: "a `Inspect<vec::IntoIter<u32>, ..>`
as an `ExactSizeIterator`, which rust-js can't count yet". The neighbour
shuffle found it where a case of that form sits beside one that reads a
capacity (#1744's two leftover pairs). The same chain of an array's items,
`.chain([9])`, compiled. MIR holds the `inspect` in a temporary while
`vec!` allocates the chain's argument. Two gaps left that temporary JS's
own map:

- `lengths_asked`'s walk gave `Held` only to a user variable; a temporary
  of an asked type went unasked. It's asked as a variable is.
- A pending temporary that `flush` writes as a `const`, where what runs
  next must not run first, skipped `written_as_asked`, which an
  assignment's write goes through. Each write of a local's value now
  reads it.

The temporary is made `$mapEnds(..)`, knowing its length, and the chain is
the `$lazyChain` the array's already was.

Case C of ADR 0262: what was refused now runs as Rust's. New corpus case
`inspect_chained_with_a_vecs_items_where_capacity_is_read` runs against
native in both builds and through every metamorphic rewrite: a chain by a
closure, by a function, of items with a destructor, of none, and an
`extend` of a `Vec` that has some. Main refuses it. Both pairs now run as
native in the plain build; they leave test/neighbours-known.txt.
`held-length-unasked` names the line as it now reads. New mutations
`held-length-temporary-unasked` and `flushed-temporary-unasked` are caught.
