# 1014. Generic code's `eq` of `TrustedLen` iterators is refused at its caller

Status: Accepted. Extends [0998](0998-eq-of-uncounted-trusted-len-iterators-is-refused.md). Amended: a zip folded or stepped too, and the guide's difference 15 gone; a side an `impl Iterator` hides, revealed (#1229).
Part of #386.

Case ([0262](0262-when-rust-and-js-disagree.md)): Rust's behavior can't be
preserved here, so it's a compile error. It was a wrong answer.

## Context

```rust
fn same<I: Iterator<Item = u32>>(a: I, b: I) -> bool { a.eq(b) }
let f = |x: &u32| { println!("a {x}"); *x };
same(vec![1, 2, 3].iter().map(f), vec![1, 2].iter().map(f))
```

std's `eq` of two `TrustedLen` iterators, `SpecIterEq`, compares their
lengths first, and steps neither of unequal ones: Rust prints `false`
alone. The generic function's JS is one for every caller and compares item
by item, as the values it's given can't report their lengths (ADR 0170).
So it printed `a 1`, `a 1`, `a 2`, `a 2`, `a 3` first. ADR 0998 refuses
such an `eq` where it's written of concrete types. Inside generic code
`I` isn't known to be `TrustedLen`, so that rule never applied there.

## Decision

An analysis, `generic_eq`, records what each body compares by `eq` or
`ne` of types holding its type parameters. It carries them to each
caller's types through generic callers, as ADR 1001 carries what's
stepped from the back. A call that makes a pair ADR 0998 would refuse
is refused at the call: both `TrustedLen`, and stepping them runs code,
a closure or a function they hold, or items with a destructor. Their
lengths are never counted before stepping in generic code.

Making it correct, the generic code given its callers' lengths, needs
iterators that carry their length, the representation change ADR 1000
decided against for now.

## Consequences

- A pair whose stepping runs nothing, `v.iter().copied()`'s, still
  compiles: comparing it item by item is unseen.
- `test/diagnostics.test.ts`' "eq in generic code of two maps" checks the
  refusal through a generic caller. The guide's limitations list it.
- The mutation is `generic-trusted-eq-allowed`.

## Amendment: a zip folded or stepped (the guide's difference 15)

The guide listed it as a difference from native Rust, though ADR 0262
never decided it: a zip in generic code makes one more of its first side's
items where its second ends first. Rust's `pairs(v.iter().map(f),
w.iter().copied())`, `a.zip(b).collect()` in a generic `pairs`, runs `f`
twice of `v`'s three items. The JS ran it three times. So do `for` loops
over it, as std's `next()` of sides it reads by index stops at the
shorter too. ADRs 0973 and 0997 made a zip rustc knows whole do as std's
does. Inside generic code that rule can't apply.

The analysis, renamed `callers_decide`, as its subject is what generic
code asks that only its callers' types answer, records a zip of its type
parameters two ways:
- folded, by any `Iterator` method but `next`, `size_hint` and `by_ref`,
  adapters too, as what they're folded by folds the zip;
- or stepped, by `next()`.

At a caller, it's refused where `zip_folds_to_the_shorter` says std's fold
stops at the shorter side. It answers `Some(true)` of sides read by index
and `Some(false)` of other `TrustedLen` ones; `folds_apart` is its
`Some(false)`. A step is refused only of sides read by index, as a
`TrustedLen` one's `next()` steps the first side once more, as JS does.
Both only where a step of the first side runs code.

- The guide's difference 15 is gone, and its limitations list it. Nothing
  observable differs: where nothing runs, the extra step isn't seen.
- `test/diagnostics.test.ts`' "a zip of a map collected in generic code"
  and "a zip of a map stepped in generic code" check it. The mutations are
  `generic-zip-fold-allowed` and `generic-zip-step-allowed`.

## Amendment: what's compared is normalized where the caller's types are made (#1182)

`callers_decide` made each ask of the caller's types without normalizing
them, so generic code's `a.into_iter().eq(b)` of `I: IntoIterator`, its
receiver `<I as IntoIterator>::IntoIter`, stayed a projection of a `Vec`
whose `TrustedLen` wasn't seen: `equal(vec![L(1), L(2)], vec![L(3), L(4),
L(5)])` wasn't refused, and its JS compared them item by item, dropping
`L(1)` and `L(3)` first, where std's `SpecIterEq` compares their lengths,
steps neither, and drops `b`'s, then `a`'s. A zip's asks were normalized
for #1162; **each ask is normalized now**, `eq`'s too, so such a call is
refused. `adapter_drops.rs` and `into_iterator_named_by_another_bound.rs`
gave generic code two `Vec`s of equal length, whose answer was std's by
luck: each gives a filter for one side now, which isn't `TrustedLen`, so
std compares item by item too; `test/diagnostics.test.ts` holds the
refusal, and `held-into-iter-drop-of-its-of`, which only that call
reached, is held by a generic `map` of an `IntoIter` held as it is,
dropped part-way. Mutation `callers-eq-unnormalized`.

Supporting std's comparison in generic code, where its callers are
seen, would take each side's length from its caller before either is
stepped, which no type fact gives: a JS iterator a caller makes, a `map`
made in place, knows no hint. And a library's generic `eq`, whose callers
it doesn't see, compares item by item whatever its consumers give it.

The readability baseline is blessed: the JS these cases no longer have,
and the filters and the `map` they have now, moved `$iter`'s share from
92.4 to 92.5 and a renamed `x$1`'s from 510.8 to 511 in 10,000 lines, no
construct of them new.

## Amended: a library's generic `eq` its consumers may give `TrustedLen` iterators (#1182)

A library doesn't see its consumers' calls, so `callers_decide` can't
refuse one: `pub fn equal<I: IntoIterator, J: IntoIterator>(a: I, b: J)`'s
`a.into_iter().eq(b)` compiled, and a consumer that gave it two `Vec`s of
unequal lengths got their items compared one by one, where std compares
the lengths first. **A library's `eq` in code its consumers may
instantiate (rustc's `reachable_set`), of two sides each of which may be
`TrustedLen`, one holding a type parameter, is refused**, an interim
refusal till generic code is given each side's length (#1170). A side may
be `TrustedLen` where it's a type parameter or an associated type, or a
type some impl of `TrustedLen` is of, a `Map`; a `Filter`, which none
is, never is, so `a.into_iter().filter(f).eq(b)` compiles.
`test/diagnostics.test.ts` holds both. Mutations
`library-eq-of-trusted-allowed` and `library-eq-of-a-filter-refused`.

## Amended (2026-11-10, #1182 (a)): a caller says whether a side's length is compared first

The two refusals above refused what std answers and JS can: itertools'
`pub fn equal<I, J>(a: I, b: J)`, whose `a.into_iter().eq(b)` is exactly
the library's shape, was refused, and with it itertools. **Each side of
generic code's `eq` given as a type parameter's, `I` or `<I as
IntoIterator>::IntoIter` (`recognition::eq_sides`), asks its caller a new
type fact, `LengthFirst`**, which a library's consumers give through its
manifest as they give `TSize`:

| Of the type a caller gives | `LengthFirst` | generic code's `$iterEqGiven` |
|---|---|---|
| not `TrustedLen` | `false` | item by item, as std's `iter_compare` |
| `TrustedLen`, its JS counted: an array given whole, an array's or a `Vec`'s iterator (`$iter`), a range of numbers (`counts_where_given`) | `true` | its `len()` or its exact hint, then std's `SpecIterEq`: lengths first |
| `TrustedLen`, its steps running nothing and its items dropping nothing | `"pure"` | collected first, then its length, unseen |
| another `TrustedLen`, a `map` whose closure runs | refused at the caller | |

`TrustedLen` is the type's, never the value's: an exact hint isn't one, a
`Skip<Chain<..>>`'s, and JS's iterators a caller makes, `v.values().map(f)`,
know no count (ADR 0170), so the caller, which knows both, decides, and the
value is read only where its caller says it counts. A side made of type
parameters, `a.into_iter().map(f).eq(b)`, stays `callers_decide`'s and the
library refusal's, as above. `test/crates/pairs/length_first_eq` holds a
library's `equal` given unequal `Vec`s, an array, a filter, a slice's
copies reversed and a range, each against native Rust;
`test/diagnostics.test.ts` the refusals that remain. The fact enters #1170's
catalogue through `TypeFact::ALL` (ADR 1115). `callers-eq-unnormalized` is
retired: the projection it held, `<I as IntoIterator>::IntoIter`, is a side
its caller's `LengthFirst` answers now, and a side made of one, a `map` of
it, is seen `TrustedLen` normalized or not; the normalizing stays, as a
zip's needs it (#1162). `adapter_drops.rs` and
`into_iterator_named_by_another_bound.rs` give their generic `eq`
`ILengthFirst` and `JLengthFirst`.


## Amended (2026-11-10, #1229): a side an `impl Iterator` hides is the type it hides

`eq_sides` matched a side's type as it's written, and an opaque one,
`impl Iterator`, a THIR's or a caller's instance of a signature's, isn't a
type parameter, a projection or a known type: asked its trait as a
projection's, rust-js crashed (found by #1204's `opaque` rewrite). Each
side is revealed first, by `mir::revealed`, the rule a body's MIR is
revealed by (#1142), in any typing environment, as std's specialization
sees the type it hides: a type parameter of the caller's it hides is
`Given`, its `LengthFirst` asked of the caller; a known one, `Known` as
any other. Generic code's `eq` of an `impl Iterator` made of its type
parameter is answered, not refused. Corpus case `eq_of_opaque_iterators`,
both builds; mutation `eq-side-of-an-opaque-type-unrevealed`.
