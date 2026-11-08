# 0057. `PartialOrd` and `Ord`: an `Ordering`, and the parts in turn

Status: Accepted. Each comparison runs its own impl, and dictionaries hold their supertraits (#937, #944, amendment below). `Reverse` compares the other way round, and another crate's struct is an error rather than field by field ([0068](0068-queues.md)). Extends [0036](0036-iterators-and-sorting.md) and [0053](0053-partial-eq.md).

Case: C, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

An `Ordering` is -1, 0 or 1 (ADR 0036), and JS's `<` compares numbers the
way Rust does. Nothing else could be compared:

- `<` on strings, `bool`s or structs;
- `cmp` of a struct;
- sorting a `Vec` of them;
- a hand-written `impl Ord`, or a generic `T: Ord`.

`sort_by_key` compared keys with `$cmp`, which is JS's `<`, so for a
tuple key it silently didn't sort.

## Decision

**`a.cmp(&b)` is an `Ordering`, and `a.partial_cmp(&b)` is one too, or
`undefined` for `None`** (`Some(x)` is `x`, ADR 0030).

| The type | `cmp` |
|---|---|
| numbers, strings, `char`s, `bool`s, `Ordering` | `$cmp(a, b)` |
| `f64`, `partial_cmp` | `$partialCmp(a, b)`: `undefined` if either is `NaN` |
| a hand-written impl | `wordOrd_cmp(a, b)` |
| a generic `T` | `TOrd.cmp(a, b)`, or `TPartialOrd.partial_cmp(a, b)` |
| derived, a struct or tuple | `$cmp(a.major, b.major) \|\| $cmp(a.minor, b.minor)` |
| derived, a fieldless enum | `$cmpIn(["Low", "Mid", "High"], a, b)`, by discriminant, as the derive compares: `High = 2, Low = 1` is `["Low", "High"]` (ADR 0089 found the declaration order used before) |
| `Option` | `None` first, then the values |
| `Vec`, slices, arrays | `$cmpItems(a, b, cmp)`: item by item, then shorter first |

- **Derived: the parts in turn, joined with `||`.** `Equal` is 0, the one
  falsy `Ordering`, so `||` gives the first part that isn't equal.
- **Where a part can be unordered** (an `f64`, or anything that isn't
  `Ord`), `undefined` is falsy too, so the parts go through `$thenCmp`
  instead, which stops at it.
- **`a < b` is `cmp < 0`,** and the same for `<=`, `>` and `>=`. Strings
  and `bool`s use JS's own `<`. For an unordered pair, `undefined < 0` is
  false, as every comparison is in Rust.
- **`a.max(b)` is `cmp > 0 ? a : b`, and `a.min(b)` is `cmp > 0 ? b : a`.**
  On a tie, Rust's `max` returns `b` and its `min` returns `a`.
- **Sorting:**
  - `v.sort()` is `v.sort(cmp)`, which is stable, as Rust's is;
  - `sort_by_key` compares the keys with their `cmp`;
  - an iterator's `max` and `min` are `$maxBy(items, cmp)` and
    `$minBy(items, cmp)`: the last of the greatest and the first of the
    least, as in Rust.
- **Dictionaries, as ADR 0049 has them:** `T: Ord` gets `{ cmp }` and
  `T: PartialOrd` gets `{ partial_cmp }`. A derived impl's dictionary is
  its comparison, or `$cmp` itself.
  - **Only an `Ord` or `PartialOrd` bound:** `a == b` was `cmp === 0`
    rather than a third dictionary, until #944: its `PartialEq`, a
    supertrait's (amendment below).
  - **A generic `max()` boxes its `Some`** if it would look like `None`
    (ADR 0051).
- **Numbers stay as they were:** JS's `<`, `Math.max`, `$cmp`.
- **Still errors:** comparing enums with fields, and `clamp`.

## Why

- **The plain case stays short.** A derived `Ord` is the comparison a
  person would write, `||` included, with no helper call.
- **`NaN` behaves.** An `f64` field makes the struct's `<` false both ways,
  as Rust's does, instead of quietly treating `NaN` as equal.
- **One function, `cmp_value`, serves operators, `max`, sorting and
  dictionaries alike,** so they can't disagree.

## Alternatives

- **A named function per derived impl** (`semverOrd_cmp`). Each comparison
  would be a call rather than an expression. It would read better where a
  struct with many fields is compared often, and it could come later.
- **Always `$thenCmp`.** It's correct everywhere, but it evaluates every
  part when `||` stops at the first difference, and it reads worse.

## Amendment: a struct of no fields

`struct Zst;` has one value, `Equal` to itself, as its derive has it, and
so has `()`, a heap of which is compared so (ADR 0068): its
comparison is `0`, which, as a part of a struct ordered field by field,
drops out (ADR 0426). It was an error, as a struct whose shape had no
parts, and stopped rustc's `zero-sized-btreemap-insert`, a `BTreeMap` keyed
by one, and `obligation-error-propagation`. The `unit_struct_order` corpus
case orders one as a map's and a set's key, compared, and as a field, with
native Rust.

## Amendment: a sort of what can be `None`

JS's `sort` moves each `undefined` to the end without asking the
comparator, so a sort of `Option`s, whose `None` is `undefined`, put
`None` last, where `Option`'s `Ord` puts it first (#101): `sort`,
`sort_by`, `sort_by_key`, and a `BTreeSet<Option<_>>` listed. Items that
can be `undefined`, those `can_be_nullish` names, are sorted by `$sort`,
which sorts their indexes, never `undefined`, by the same comparator and
puts the items in that order: `$sort(scores, (a, b) => …)`. Every other
sort is JS's own `items.sort(…)`, as before; `sort_by_compare` is the one
place that picks. ADR 0262's case: results would change, so Rust's. Held
by `test/corpus/option_sort.rs` and the mutations `nullable-sort-by-js`
and `sorted-keys-by-js`.

## Amendment: a type compared with another

`ordering_call` ordered the type with itself, as `PartialOrd<Self>` is,
whatever the trait's `Rhs`: `m < 4` of a struct with its own
`PartialOrd<u32>` was `$cmp(m[0], 4[0]) < 0`, field 0 of a number, false
(#106). Of a crate's own impl whose `Rhs` isn't `Self`, `<`, `<=`, `>`
and `>=` are std's defaults, its `partial_cmp` against 0, which `None`'s
`undefined` makes false: `metersPartialOrdU32_partialCmp(m, 4) < 0`; its
`partial_cmp` is called as its impl's, as before. ADR 0262's case:
results would change, so Rust's. Held by
`test/corpus/partial_ord_other_type.rs` and the mutation
`ordering-of-other-type-as-self`.

## Amendment: the derived order of a type inside itself

A derived order is its parts' orders inlined, so of a type inside itself,
`enum V { A, C(Box<V>) }` or `struct L { next: Option<Box<L>> }`, it was
inlined without end and the compiler overflowed its stack (#148). Its
order is a function, `const cmpTree = (a, b) => ..`, which calls itself
for the ones inside, as a recursive type's clone is (`clone_value`); each
place that orders one makes its own, as each clone does. Held by
`test/corpus/recursive_order.rs`.

## Amendment: a B-tree's order

Comparing B-trees, `a < b` of two `BTreeSet`s or `BTreeMap::cmp`, was
refused (#165). A B-tree compares as its items in its order, as a
sequence does (`$cmpItems`), a map's its entries, key then value; its
keys' own `Ord` orders them, and `f64` values make it partial. Held by
`test/corpus/btree_compare.rs`.

## Amended by

[ADR 0687](0687-a-consumer-steps-its-iterator-as-std-does.md): `max()`,
`min()` and the `_by` and `_by_key` ones are given the iterator, not its
array, and step it as std's `reduce` does, each item as it comes.

## Amendment: another type's order through references (#467)

chrono's `map_local`, `.filter(|dt| dt >= &DateTime::<Utc>::MIN_UTC)` of
a `&DateTime<Tz>` in code generic over `Tz`, was refused, "calling
`PartialOrd::ge`", so chrono compiled to its types alone and an app's
`NaiveDate::parse_from_str` was refused with it. It compiled before
8635ec3e2 (#106), whose branch for a type ordered by another's
`PartialOrd<B>` asked `is_user_impl` of the trait as called, `<&At<Z> as
PartialOrd<&At<Utc>>>`, which is std's `impl PartialOrd<&B> for &A`, not
the crate's. A regression; ADR 0262's case: Rust's behavior is kept, and
a refusal is lifted.

Of references, that order is its referents', `A: PartialOrd<B>`'s, the
trait taken through them before its impl is asked for, or its dictionary
in generic code: `&at >= &MIN` is `atPartialOrdAt_partialCmp(at, MIN) >=
0`, as before #106, and `fn less<A: PartialOrd<B>, B>(a: &A, b: &B) ->
bool { a < b }`, which the limitations listed, compiles.

- `test/corpus/partial_ord_other_through_refs.rs` checks `>=`, `<=`, `<`
  and `>` of `&At<Z>` and `&At<Utc>` in generic code against native Rust;
  `test/limitations.test.ts` lost its line.
- The mutation is `ordering-other-through-refs-unpeeled`
  (`lower/ordering.rs`).

## Amendment: `max`, `min` and `clamp` as std's are (#912)

`a.max(b)` was `cmp(a, b) > 0 ? a : b`: a comparison that prints said
`cmp 1 2` where std's `Ord::max`, `if other < self { self } else { other }`,
says `cmp 2 1`; and of a type with a destructor, the one not given back was
never dropped, nor, as the comparison panicked, either, where std drops its
parameters, the later first. Results change (ADR 0262): Rust's behaviour is
kept.

- **The order is std's:** `max` and `min` ask `cmp(b, a) < 0`, std's
  `other < self`; `clamp` asks `min <= max`, then `self < min`, then
  `self > max`, as it did. Of a type with nothing to drop, the conditional
  it always was, the comparison's arguments the other way round:
  `rankOrd_cmp(b, a) < 0 ? a : b`. Of numbers and text whose comparison
  can't be seen, the order changes nothing (ADR 0262 "none"), and the one
  form is kept for all.
- **What's not given back is dropped:** of a type with a destructor,
  `$maxOf(a, b, cmp, drop)`, `$minOf`, and `$clampBy(x, min, max, cmp,
  show, drop)`: the one, or two, not given back dropped as std's frame ends,
  `max` before `min` before `self`; and as a comparison, or `clamp`'s
  bounds' assertion, panics, every parameter, the later first (`$unwinds`).
  A drop that panics after the choice leaks the one given back, as rustc
  does (ADR 0465's #648 amendment).

`std::cmp::max_by`, `min_by`, `max_by_key` and `min_by_key` are refused, as
they were. The corpus's `max_min_clamp_as_std_compares` runs each of them
under each panic point against native Rust; `test/lowering-std-unwinds.test.ts`
pins both forms; the unwind table's 76 case-builds of #912 pass.

## Amendment: each comparison runs its own impl (#937, #944)

ADR 0057 took a type's `PartialOrd` and `PartialEq` to follow its `Ord`,
which Rust never ties: `partial_cmp` of a derived `PartialOrd` beside a
hand-written `Ord` called that `cmp`, where rustc's derive compares the
fields (#937), and `cmp` of a derived `Ord` beside a hand-written
`PartialOrd` called its `partial_cmp`; std's sorts, heaps, `max`, `min` and
`clamp`, which ask `<` and `<=`, asked `cmp`; and generic code's `a == b`
and `a < b` of a `T: Ord` were `TOrd.cmp(a, b) === 0` and `< 0` (#944).
Each answered by the other order, or called other methods than Rust's,
which a comparison that prints shows. Results change (ADR 0262): Rust's
behaviour is kept.

- **One owner, `ordering.rs`, says what each runs, from rustc's impls.**
  `partial_runs`: a hand-written `partial_cmp`; its `cmp`, of one that is
  `Some(self.cmp(other))`, rustc's derive's beside a derived `Ord` (in the
  same `#[derive]`, of a type without parameters, which rustc decides; read
  from the impl's HIR, never guessed) or a hand-written one, which does
  nothing else, so calling `cmp` in its place changes nothing (ADR 0262
  "none"); else its parts' `partial_cmp`s. `cmp` is a hand-written `cmp`
  or its parts'. `operator_value` says what `<`, `<=`, `>` and `>=` run:
  JS's own of numbers, an impl's own `lt` where it writes one, else
  `partial_cmp` against 0. Sorts' `is_less`, `max`'s and `min`'s `other <
  self`, JS's `sort` comparator (`partial_cmp`'s) and `sort_by_key`'s keys
  ask it.
- **Dictionaries hold their supertraits, as Rust's do** (the user's choice,
  form A): every `Ord` dictionary has `PartialOrd()`, and every
  `PartialOrd` dictionary `PartialEq()` and `lt`, `le`, `gt` and `ge`; the
  route to a supertrait is rustc's (`holds_supertraits`), so generic code
  reads `TOrd.PartialOrd().lt(a, b)`, `TOrd.PartialOrd().PartialEq().eq(a,
  b)` and `TOrd.PartialOrd().partialCmp(a, b)`, never `cmp`, and no
  fallback. `$partialOrd({ PartialEq, partialCmp, lt? })` gives std's
  provided operators over `partialCmp` where an impl writes none, the one
  place those defaults are; an impl's own are in its dictionary. std's
  quiet orders, numbers but floats, text, `char`s and `bool`s, share
  `$ordered` and `$partiallyOrdered`; a derived or std type's of the
  module's types is its module's `const`, `pairPartialOrd`, made once and
  read wherever it's given, as an impl's is (`recursive_fns.rs`'s
  `module_dictionary`, the user's ask).
- **std's helpers that ask operators are given the `PartialOrd`
  dictionary**, so each runs what std's MIR runs there, an impl's own
  operator included: a heap's sifts (`le`, `ge`, `lt`), `clamp` (`le`,
  `lt`, `gt`), `max` and `min` of a type with a destructor (`lt`), a
  B-tree's range check (`gt`), `extract_if`'s end (`le`, `lt`) and a set's
  bulk build (`lt`). A sort of a type whose impl writes its own `lt` is
  std's `$sortStable`, as JS's `sort` takes an `Ordering`.

The corpus's `partial_ord_apart_from_ord` (#937) and
`generic_compare_by_impls` (#944) run each against native Rust;
`test/lowering-dictionaries.test.ts` pins the dictionaries' forms. The
mutations are in `ordering.ts`, `std_impls.ts`, `traits/impls.ts`,
`traits/dictionaries.ts`, `recursive_fns.ts`, `slice.ts`, `heap.ts`,
`ends.ts`, `runtime/partial_ord.ts` and `runtime/sift_down.ts`. Left, filed
P0 as #962: an impl's own `ne`, `max`, `min` or `clamp` called from generic
code or std's helpers runs std's.
