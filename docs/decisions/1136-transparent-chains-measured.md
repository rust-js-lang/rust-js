# 1136. Transparent iterator chains, measured

Status: Accepted (2026-11-13, #1673, campaign #1681). A measurement: it
decides nothing a program does. Review #1671's change 2, "std's own MIR
is the meaning", starts from it; the migration it informs is its own ADR.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. The query only
counts, where decisions are asked (`RUST_JS_DECISIONS`, ADR 1124).

## Context

rust-js writes most iterator chains by hand, as JS arrays, iterators and
runtime helpers, and keeps std's compiled adapters (`--std-iterators`) as
the oracle they're proved against. The hand-written forms are where the
iterator wrong answers come from: their order, hints, capacities, drops
and back-stepping are rust-js's own account of std's, and each new
combination needs its own proof. Change 2 makes std's compiled body the
default and keeps the hand-written form only where it's known equal.

Which chains is that? Review #1671 proposed `transparent(chain)`: every
link a std adapter over an array, a range or a string; no `Iterator` impl
of the crate's reachable; no drop glue in the items or captures; nothing
escapes.

## Decision

**`iterators/transparent.rs`' `count_chains` answers, of each chain, the
first reason it isn't transparent, counted as the decision
`transparent-chain`:** `user-iterator` (a type of the crate's that's an
`Iterator` among its links' generic arguments), `held` (it starts on an
iterator held apart: a variable's, a field's, a parameter's or a crate
function's), `source` (it starts on something other than an array, a
`Vec`, a slice, a string or a range), `escapes` (the iterator it makes
outlives the expression, not a `for` loop's), `drop-glue` (an item, or a
closure's capture, has a destructor), or `admitted`. A chain is a std
call that makes or steps an iterator, followed through its first argument
to what it starts on; a `for` loop's own `next` isn't one. It's asked in
`body_queries/in_place.rs`' `decide`, of the plain build only, and only
where decisions are asked. Held by `test/decisions.test.ts`' "each chain
is counted by what keeps it from being transparent" and nine mutations.

## The measurement (2026-11-13, at 26b9a8eec plus this)

**The corpus**, 2,411 cases compiled (40 compile-fail or ignored):

| Answer | Chains |
|---|---|
| admitted | 3,186 (26%) |
| held | 3,206 |
| escapes | 1,754 |
| user-iterator | 1,751 |
| source | 1,126 |
| drop-glue | 1,003 |
| total | 12,026 |

1,529 cases have a chain; 564 have only admitted ones.

**The React port** (react-in-rust at its pin), 541 chains: admitted 329
(61%), held 123, drop-glue 36, escapes 35, source 18, user-iterator 0.
Ports read more like the predicate's target than the corpus, which tests
the edges on purpose.

**What std's compiled body costs in readability.** The JS of the 965
corpus cases with a non-admitted chain is 181,363 lines in the plain
build and 232,652 in the std build, 1.28 times; of the 564 cases whose
chains are all admitted, 1.02 times. That is a whole-case measure of every
chain going to std's body, an upper bound on the migration's cost, which
moves only the non-admitted chains.

**The last 100 iterator wrong answers** (`P0: wrong answer` and `area:
iterators`, #826 to #1667). 86 have the corpus cases their fix added,
found by the commit that names them (143 cases); 14 were fixed without
one added by such a commit. Of the 86:

- 83 are in cases with a chain the predicate doesn't admit. Their first
  reason, by the reason most of the case's chains give: held 23,
  user-iterator 22, escapes 16, source 13, drop-glue 9.
- 2 are in cases whose chains are all admitted: #1015, `inspect(fn_item)`
  run before a `chain`'s argument is evaluated, and #1635, a `Vec`
  collected from `cycle().take(n)` with the wrong capacity. Each is a
  hand-written form wrong on a chain the predicate admits: the order of
  evaluation and a collect's capacity are observable through admitted
  chains, so the predicate as written doesn't make a chain's hand-written
  form safe by itself.
- 1 has no chain in its cases (#1023, a library type's own `Iterator`
  methods).

The attribution is by case, not by the chain that was wrong: a wrong
answer's case holds 81% non-admitted chains on average, against 51% across
the corpus; 44% of them have no admitted chain at all, against 34%; and
15% of their chains are admitted, against 26% overall.

## Consequences

- The wrong answers sit where the predicate says std's body should be
  the meaning: 83 of 86. Moving non-admitted chains to std's compiled body
  would have kept most of them from being written.
- The predicate is necessary, not sufficient: #1015 and #1635 show the
  hand-written form of an admitted chain can still be wrong, on the
  evaluation order of an adapter's argument and on a collect's capacity.
  The migration's predicate needs those two too, or the admitted
  hand-written forms need proving against std's bodies, as the conformance
  table does.
- `held` is the largest non-admitted reason in both the corpus and the
  port, and std's compiled struct is already what a held adapter is (ADR
  0871), so it's the cheapest first family.

## Amendment: admission tightened (2026-11-13, #1681's change 2, step 2)

Still a measurement: nothing reads `transparent-chain` yet. The held
family's step is the first to.

- **A link is an iterator method, or a maker of an iterator from what
  isn't one.** A method of `Iterator` or of a trait over it
  (`DoubleEndedIterator`, `ExactSizeIterator`), or a std function that
  makes an iterator of what isn't one, a source's (`v.iter()`, `s.chars()`,
  a `for` loop's `into_iter`). Not one that makes a range, which is the
  source itself (`RangeInclusive::new` of `1..=3`), nor what's given an
  iterator to read, `{:?}`'s `Argument::new_debug`.
- **`unproved`: an iterator method no table proves.** `PROVED_LINKS`
  (`recognition/compiled_std.rs`), resolved once with the catalog's rows
  (ADR 0561, amended), lists the methods whose hand-written form the
  iterator tables run against native Rust: 22 adapters and 18 steps.
  `test/iterator-proved.test.ts` holds each to a row of
  `test/iterator-matrix.ts`, the mirror held by running both.
- **`order`: an adapter's argument evaluated as it's given.** Any argument
  but the first that isn't inert: a closure, a function item, a literal or
  a constant, a variable, a place of one, or a value made of those, a
  range's `..`. A hand-written form may evaluate it at another time than
  Rust does (#1015).
- **`capacity`: a `Vec` collected in a crate that reads capacities**,
  whose capacity std's hint makes (#1635).

Measured at this commit: the corpus's 11,128 chains (fewer than above, as
links are narrower) give admitted 2,483, held 2,908, escapes 1,758,
user-iterator 1,674, drop-glue 1,054, source 869, unproved 177, order 155,
capacity 50. The React port's 513 give admitted 300, held 123, drop-glue
36, escapes 35, source 17, order 1, unproved 1. #1015's case now answers
`order` and #1635's `unproved` (`cycle`): neither is admitted. Of the
port's chains this step keeps out, two have no proof yet, which Tu's
choice (b) asks for before they'd move: `call_sites.iter().skip(
FRAMES_TO_SKIP.get())` (ReactFlightStackConfigV8.rs:64), an argument that
is a call, and `a.encode_utf16().cmp(b.encode_utf16())`
(ReactStrictModeWarnings.rs:93), `Iterator::cmp`, of no table's row.
Mutations of each answer, caught by `test/decisions.test.ts`.

## Amendment: two sources of a `Box<dyn Iterator>` in the tables (2026-11-13, #1681's held step (i))

The React port's held chains, 118 of 123, start on a `Box<dyn Iterator>`
a call gives, most a binding's JS iterable (webapi's `keys()`, `entries()`,
an element list's). No table had one. The matrix, the conformance table
and the points table gain two sources, appended so no existing case moves:

- **`boxed dyn`**: `boxed(ds())`, a `Box<dyn Iterator<Item = D>>` a function
  returns of a `Vec`'s, its items printing as they drop.
- **`js values`**: `js_values(&nums)`, a binding (`link_name = "values"`)
  that gives JS's own iterator of an array behind the box. Natively it's an
  iterator of the same items that knows no hint, `std::iter::from_fn`, as
  a JS iterator doesn't (`(0, None)`).

They found two wrong answers, filed first as P0s and listed in
`test/iterator-matrix-known.txt` under them:

- **#1690**: a `Box<dyn Iterator>` of a Rust iterator, unwinding from a
  step, drops what it still holds before what native drops first: the
  consumer's accumulator, a closure's capture. In both builds, and a drop
  panicking while unwinding doesn't abort. 502 case-builds.
- **#1691**: `take(n)` of an iterator with no upper bound hints `(0,
  None)` in the plain build, where std's `Take` hints `(0, Some(n))`. 2
  case-builds.

Every other case of both sources runs as native Rust does, in both builds:
each adapter and consumer, at each panic site and point. So a binding's
JS iterator behind a box is proved but for `take`'s hint (#1691), and a
Rust iterator behind one isn't, until #1690 is fixed.

## Amendment: a binding's boxed iterator is a source (2026-11-14, #1681's step ii)

A chain may start on a binding's boxed iterator, `Box<dyn Iterator>` a
binding's call gives, webapi's `keys()`, `values()` and `entries()`: JS's
own iterator, whose steps run no Rust, so nothing of the crate's is
reachable through it, as nothing is through an array's. It's counted as
the array is, where it was `held`. A boxed iterator Rust makes, a
function's `Box::new(items.into_iter())`, webapi's own `iter()` of a
`DOMTokenList` among them, stays `held`: its `dyn` hides what it boxes,
which may be an `Iterator` of the crate's, `Box::new(Counter(0))`, whose
code a transparent chain never reaches. (A boxed iterator's own steps are
proved, both sources in the iterator tables, since #1690 and #1691.) Still read-only (step iii is what reads it).
In the corpus, its one such chain,
`take_of_an_iterator_with_no_upper_bound_hints_its_count`'s, is admitted.
Held by `test/decisions.test.ts`' count, a binding's chain admitted and a
Rust-made boxed one `held`; mutation `binding-boxed-source-held`, caught,
and `transparent-held-unseen` retargeted, caught.

## Amendment: one named dictionary of a JS iterator's steps (2026-11-14, #1681's step iii)

Generic code given a JS iterator, a type parameter's, a binding's or one
std's compiled struct steps, was given its `Iterator` dictionary as an
object of the runtime's steps, written out each time, `{ next: $next, fold:
$foldOf, nth: $nth, last: $last, count: $count }`, or its boxing form of
items that look like `None`. Each is now one named object of the runtime's,
`$jsIterator` or `$jsIteratorSome`, frozen: the same entries, each of which
asks what the iterator has of its own, else steps it as std's default does.
A dictionary any entry of which differs, the crate's own, or one that leaves
an entry out (`{ next: $next, fold: $foldOf }`, of an iterator whose `nth`
`$nth` would call it hasn't), keeps its own object. 31 corpus snapshots move,
a net 338 lines fewer; the held flip's step gives each std struct over a JS
iterator the name rather than the object. Mutations
`js-iterator-dictionary-inline` and `js-iterator-some-dictionary-inline`,
caught by their cases' snapshots. The readability measure's shares of
renamed `x$1` (483.5 to 483.6) and role fallbacks (264.2 to 264.4) in
10,000 lines rise only as the lines they're counted against fall, the
same names in 338 fewer lines; its baseline is blessed so.

## Amendment: the held flip (2026-11-14, #1681's step iii)

The first step that reads `transparent-chain`. A chain that starts on a
variable holding std's struct apart, `held`'s, is std's compiled structs:
its `enumerate()`s and `map()`s, `HELD_FLIPPED`, are no longer made in
place, the recognizers' hand-written forms, but held, std's body the
meaning where the hand-written form isn't proved to be it. Decided as the
second stage of `InPlace` (`decide_held`), once `back_stepped` knows what's
stepped from its back and before `type_fact_params` reads the sets.

- **Narrowed to std's struct.** A source made knowing its back or its
  length where it's made, `$mapEnds` of a map a `rev()` or a counted
  enumeration steps (#484, #1354), is a JS iterator, not std's struct, and
  its chain keeps its forms: `BackStepped::made_knowing`, the owner's
  answer, of its type or a local of it. Variables only, not fields. Unnarrowed,
  the flip moved 175 snapshots, ×1.146 their lines, and refused one,
  `held_filter_map_reversed`.
- **Never `rev()`.** `back_stepped`, which decides the flip, reads whether a
  `rev()` is made in place: its held-`rev()` rule marks what a held one is
  given made knowing its back (analysis/back_stepped.rs). Flipping a `rev()`
  would mark its own source, un-flipping the chain: a cycle, not a fixed
  point. `test/architecture-iterators.test.ts` holds `HELD_FLIPPED` to
  `enumerate` and `map` and back_stepped's read to the ground.

Measured: 49 corpus snapshots move, each as native prints; their lines
11,151 to 11,716, ×1.051 (×1.065 before `$jsIterator`, which shrank the
dictionaries the std structs are given). Worst: `associated_into_iter`
×1.29, std's `iterator_map(iters, f, $jsIteratorSome)` where JS's own wrote
`Iterator.from(iters).map(f)`. The React port's regeneration changes no
iterator form. The readability measure grows in no shape. Mutations
`held-flip-takes-none-out` and `held-flip-of-a-source-made-knowing-its-back`,
caught by snapshots; `held-stage-never-decided` retargeted, caught.

## Amendment: the held flip reverted (2026-11-14)

The held flip (bb518fcca) is reverted: it made a wrong answer of a refusal.
Of the matrix's generic sources, `src.chain(v).map(f).flatten().rfold(..)`,
its flipped `map`, std's compiled `Map`, was given its source, a chain made
in place, the runtime's `$lazyChain`, with a dictionary that steps it from
its back as std's compiled `Chain` struct, which a JS chain isn't: it
stepped nothing, `rfold 0 after 0` where native's is `rfold 126 after 14`,
and where the hand-written form refused it ("a `Chain<I, IntoIter<D>>` as a
`DoubleEndedIterator`"). A flip must make what a compiled adapter is given
std's struct too, as ADR 0871 says of what's given to a compiled body, or
refuse; and its push must run every matrix and conformance row its decision
reaches, by source, not only the rows the adapters it names reach. Re-landed
with both. The readability baseline, blessed since over the flip's forms, takes
back the shares they had: `Iterator.from` 7.7 to 8.5 and `$iter` 84.2 to
84.3 in 10,000 lines, the pre-flip forms', each as native prints.

## Amendment: the held flip re-landed, from the source outward (2026-11-14, #1780)

The held flip is back, as its amendment above described it (narrowed to
std's struct, `BackStepped::made_knowing`; never `rev()`), with the rule
whose absence made #1780's wrong answers: **what a compiled adapter is
given is std's struct**, as ADR 0871 says of what's given to a compiled
body. A held chain's links are flipped from its source outward, each while
it's a `HELD_FLIPPED` one, and none past a link that isn't (`held_links`'s
`map_while`): `src.chain(v).map(f)`'s `map` is given a `$lazyChain` made
in place, which the compiled `Map` stepped as std's `Chain` struct,
folding nothing; `src.take(n).map(f)`'s was given a hand-written `take`,
and dropped twice more. Both are the hand-written forms now, as before the
flip, and `src.map(f).enumerate()` is std's compiled structs, each given
std's struct. `test/architecture-iterators.test.ts` holds the rule.

Proved before its push: each row the push slice picks of a change to what
decides a chain's form (ADR 1090, amended), 82,926 of the matrix's and
14,144 of the table's, the whole of `pairs`, 12,000, and #1780's six
rows, all passing. 47 corpus snapshots move, each as native prints, their
lines 10,466 to 10,949, ×1.046 (×1.051 unruled); worst
`associated_into_iter` ×1.29, as before. The React port's regeneration is
byte for byte what its pin committed; readability grows in no shape.
#1574's neighbour refusal ends again. New corpus case
held_chain_mapped_past_a_link_not_compiled (`chain`, `take` and `step_by`
then `map` then `rfold`, with drops; `flatten` then `map`; a compiled
`map` then `enumerate`); mutation held-flip-past-a-link-not-flipped, the
every-link rule, caught by it, beside the flip's three, caught. Three
mutations whose cases the flip made std's compiled structs, so they no
longer reached the hand-written form they break, are kept caught by cases
it leaves hand-written: cloned-owners-bounds-unasked and
cloned-owners-unrecorded by generic_map_past_a_take_beside_a_cloned_one,
#1318's case with each `map` past a `take`; closing-try-fold-by-next by
skip_try_fold's `any_kept_above`, a `map` past a `filter`; and, found
after, generic-instance-unknown-bound-unmet (`instance_within_bounds`'s
bound no parameter decides, #955, #1109) by
generic_map_past_a_take_cloned_beside_another_bound, its case's `map` past
a `take`.

## Amendment: two reasons asked of their owners (2026-11-14, #1681)

Two of the predicate's reasons approximated questions another module
answers, and kept out React's everyday chains, which the map and enumerate
flip would then have moved: `invalidProps.map(f).join(", ")` as
`$foldToArray(iterator_map($iter(invalidProps), f))`.

- **`drop-glue`** asked rustc's `needs_drop`, which is true of a `String`'s
  or a `Vec`'s items, whose freeing JS never sees. It asks the drops owner
  now, `drop_runs`: whether a user `Drop` runs.
- **`capacity`** kept out every `Vec` collected in a crate that reads
  capacities. It asks the capacity owner now (#1180, #1200): whether the
  collected iterator is collected knowing its exact length
  (`grows_by_exact_len`) or its hand-written hint is std's lower bound
  (`lower_bound_is_stds`, `lower_bound_kept_by_parts`). An exact-sized
  `v.iter().map(f)`'s or `(0..n).map(f)`'s is.

The chains are counted where the flip reads them, `decide_held`, once
`drop_runs` exists, not in the ground's pass: one predicate, asked in one
place. Byte-identical: no corpus snapshot and no file of the React port
moves (the flip's set, `held`'s, comes before both reasons). The decisions
test's count keeps one chain of each, a user `Drop`'s items, `Loud`'s, and
a `chars()` collect whose hint isn't std's; mutations
`drop-glue-admits-a-user-drop` and `capacity-admits-an-inexact-hint`,
caught, and `transparent-capacity-unseen` and `held-stage-never-decided`
retargeted, caught.

## Amendment: the map and enumerate flip widened, every reason but admitted (2026-11-15, #1681's step (b))

The held flip took a held chain's `map()`s and `enumerate()`s out of the
recognizers' hand-written forms. It takes them out of every chain the
predicate doesn't admit now, whatever keeps it out, as std's compiled
structs, from its source outward (the re-land's rule): each a chain whose
hand-written form no table proves to be std's body. Not of one whose
source, or whose own value, is made knowing its back or its length where
it's held (`BackStepped::made_knowing`): `let shifted = (1..6).map(f);
shifted.enumerate().rfold(..)` keeps its `$mapEnds`, which std's struct
would be given that knowing only by each of std's back dictionaries, three
times its JS.

Two of the predicate's reasons over-excluded, and were fixed first, so no
port shape moves:

- **`extend` is a consumer.** A chain given to std's `Extend::extend` of a
  `Vec`, a `VecDeque`, a `String` or a `HashMap` is consumed where it's
  made, as one collected is, not escaping; a `Vec`'s asks the capacity
  reason as a collect's does. React's `rects.extend((0..n).map(f))` is
  admitted, `$lazyRange(0, n).map(f)` as it was. The rows that prove
  `$extend`'s hand-written form, of every source and chain, landed with
  #1803, #1804 and #1805, the three wrong answers they found (ADRs 0419,
  0991, 1090).
- **A compiled struct given to generic code is given only what it asks.**
  `knowing_held` gave std's compiled struct its back, `next_back`, its
  `rfold`, `nth_back` and `try_rfold` and std's `DoubleEndedIterator`
  dictionaries, wherever its type had one, to generic code bound by
  `ExactSizeIterator` alone too, which can't step it from there: now only
  where it's asked (`as_exact_size`'s `back`).

Measured: 169 corpus snapshots move, each as native prints, ×1.050 their
lines (55,131 to 57,877); worst enumerate_of_take_from_back ×1.88, a
range's map of items with a destructor collected, `drop-glue`'s, std's
compiled `Map` where `$range(1, 7).map(f)` read as JS (#1826, the next
step: its hand-written form proved by the tables' `drop` and `stage` rows).
A refusal ends: a map of a range stepped from its end given to generic
code that asks its length, its hand-written form refused (ADR 0987), is
std's compiled structs (corpus case
map_of_a_reversed_range_given_for_its_length, against native Rust, in
place of its diagnostics row; mutation reversal-of-range-chain-as-items
retargeted, caught). The stream-order test's console, the browser's, holds
std's compiled bodies beside the helpers, as `@rust-js/std`'s entry
re-exports them. The
React port's JS regenerates byte for byte. Readability reads better in two
shapes (renamed `x$1` 477.9 to 474.7, role fallbacks 260.9 to 259.1 in
10,000 lines). The decisions test counts a range's map extended, admitted,
and a `chars()`'s, a capacity. Mutations transparent-extend-escapes,
transparent-extend-capacity-unasked, knowing-held-back-unasked and
widened-flip-of-a-chain-made-knowing, caught; transparent-escapes-unseen,
transparent-for-loop-escapes, transparent-capacity-unseen and
capacity-admits-an-inexact-hint retargeted, caught.

## Amendment: drop-glue retired, the item shapes proved (2026-11-15, #1826)

`drop-glue` kept a chain out where an item, or a closure's capture, has a
destructor JS runs, the drops owner's `drop_runs`: its hand-written form
wasn't proved to drop as std's body does. The tables prove it now. The
matrix, the conformance table and the panic points take four nested item
kinds beside `D`, `&D` and `String`: an `Option<D>`'s, a `Vec<D>`'s, a
`(D, D)`'s and a struct's that holds a `D` (sources `options`, `vecs`,
`d pairs`, `helds`), and the adapter `map made`, `.map(|x| D(x.v() + 100))`,
over each item kind. Each kind is tested at the drop, stage and consumer
sites, with closures that capture a `D`, in both builds, the twin net's
too: 30,920, 7,766 and 2,561 new cases.

Run with `drop-glue` kept and retired, the new rows fail the same 439
case-builds, none of them caused by admission. Grouped by cause:

- **Two wrong answers**, fixed first. #1854: what a `Vec`'s `IntoIter`
  passes over from its back dropped back to front, of items whose drop is
  their glue (ADR 0465, amended: what drops as an array's items is
  `$dropEach`, by its type). #1855: a `rev()` a step from its back is
  given, reversed inward into JS's own chain, which has no back, threw
  `it[by] is not a function` (ADR 0516, amended). With those fixed, 322
  remain.
- **282 refused at run time** under #1180: `capacity()` of a `Vec` std may
  collect in place, whose capacity JS doesn't keep, of each nested kind.
- **40 the guide's difference 2** (#1651), listed under `#difference 2`:
  a `Vec<String>`'s or a `Vec<Vec<D>>`'s items mapped to `u32`s or `D`s,
  which std collects in place on wasm32, whose layouts are rust-js's
  (ADR 0090), and not on a 64-bit machine. So the closure, what's left and
  the capacity drop and read as on wasm32 (#1856, closed so). The same
  collect into `usize`s, in place on both, runs as native Rust does.
  test/iterator-matrix-known.txt takes `#difference <n>` headings as
  test/metamorphic-known.txt does, read by one reader of both,
  `knownHeading` (test/support.ts).

So the reason is retired: `Transparency::DropGlue`, `drops_in` and the
predicate's `drops` are gone, and `decide_held` no longer asks
`drop_runs`. The decisions test's chain of a closure holding a `Loud` and
its `Loud` items are admitted (12 admitted, none `drop-glue`). Mutations
`drop-glue-admits-a-user-drop`, `transparent-item-drops-unseen` and
`transparent-capture-drops-unseen` are retired with the reason they held.

Measured: 121 corpus snapshots move, each as native prints, 45,166 lines
to 43,162 (×0.956). 70 of the 171 that step (b) moved are byte for byte
what they were before it. 26 more of (b)'s move part of the way back.
25 outside
(b) move by #1854's `$dropEach`. The React port's JS regenerates byte for
byte. Readability's shares are blessed a little up, as their counts held
while the corpus's JS shrank by 1,647 lines: `while (true)` 224 both, 6.5
to 6.6 in 10,000 lines; `$iter` 2,868 to 2,875, `renamed x$1` 16,214 to
16,166 and role fallbacks 8,850 to 8,866, the new cases' 7, none and 15.
The new rows take two helpers both builds import into the twin net's
`shared`, `$nextBackSome` and `$peekSome`, the boxed forms (ADR 0051) of
`$nextSome`'s and `$peek`'s. With std's iterator structs (ADR 0908),
reversed_map_stepped_from_its_back's `try_rfold` of an `Option` is refused,
std's `Try` of one not given to its generic code yet (#1864, in
iterator-forms' `differs`).

## Amendment: the chain family flipped (2026-11-15, #1681's change 2)

The first of the families after `map` and `enumerate`: a `chain()` of a
chain the predicate doesn't admit is std's compiled `Chain` from its
source outward, `HELD_FLIPPED`'s, no longer the recognizers' `$lazyChain`
there. A flipped `map` past an unflipped `chain` was what #1780 found, its
source given as std's `Chain` struct, which it wasn't: with `chain`
flipped too the walk from the source goes through it.

Measured: 40 corpus snapshots move, each as native prints, 16,925 lines
to 17,499 (×1.034), the corpus as a whole +574 of 328,075. The React
port's JS regenerates byte for byte. `@rust-js/std` is the same size,
1,307,950 bytes, as std's `Chain` was compiled into it already. The
iterator matrix's and the conformance table's slices pass. Readability
reads a little better in each share: `$iter` 84.1 to 83.9,
`Iterator.from` 8.3 to 8.1, renamed `x$1` 468.8 to 468.7 and role
fallbacks 256.8 to 256.7 in 10,000 lines. Mutation `chain-not-flipped`
(`lower/body_queries/in_place.ts`).


## Amendment: the zip family flipped, and nothing flipped is given a JS iterator (2026-11-16, #1681's change 2)

A `zip()` of a chain the predicate doesn't admit is std's compiled `Zip`,
`HELD_FLIPPED`'s, no longer the recognizers' `$trustedZip` or `$lazyZip`
there. Flipping it found a wrong answer the walk from the source had
left open since `map` and `enumerate` flipped. A variable a `let` binds
to an adapter, `let a = v.iter().map(f)`, is made in place by the ground
where every read steps it as a JS iterator, and the zip it's given to was
such a step; the flip then took the zip out of the ground but not the
variable's map. Std's `Zip` read that JS iterator by index, through the
compiled `Map`'s `__iterator_get_unchecked`, and `a.zip(0..10).fold(..)`
summed 0 where Rust sums 29 (and the minified build of
`zip_by_index_of_holding_maps` folded nothing). Results changed, so this
follows ADR 0262's third case. The flipped `chain` and `map` had the same
hole, hidden because their fold steps any JS iterator.

So a link is flipped only where nothing it's given, its receiver or
another iterator, `zip`'s `b` or `chain`'s, is a JS iterator the ground
made: a call made in place there and not flipped, or a variable `let_made`
binds to one (`held_links`). What's past it isn't flipped either, as
before.

The rule: **a zip whose sides are type parameters stays the hand form**
(the lead, 2026-11-16). Nor, that is, is a zip of sides only its callers
know, a type parameter's or one built of one (`zip_asks_its_callers`),
flipped. std's `Zip` specializes on whether
each side is `TrustedRandomAccess` or `TrustedLen`, which rustc can't say
there, so its compiled body asks each as a fact the callers give: three
parameters more on a generic function, `dot(it, other, IRandomAccess,
IRandomAccessNoCoerce, ITrustedLen)` where `dot(it, other)` was, and none
given to a trait's default compiled over `Self`, itertools' blanket
`impl<T: Iterator> Tools for T`'s `dot`, which was refused (the crates
pair `library_instance_impl_asks_its_defaults_facts`). The recognizers'
`$lazyZip` asks neither and is proved by the tables, so such a zip stays
theirs (corpus `zip_of_sides_only_callers_know`). The build of std's
structs, which compiles that zip anyway, still refuses it in the default,
on `iterator-forms`' `differs` list (#1956).

No program on main answered wrongly: its flipped `chain` and `map`, given
such a JS iterator, folded it through `$foldOf`, which steps any. Mutation
`flipped-given-a-let-made-in-place` checks the snapshots of
`iter_by_ref`'s `map` and `adapter_source_drops`' `chain` too, which stay
the hand form, so it is caught there whatever `$foldOf` steps. `let_made` is the one answer of which variables a `let` binds to
an adapter, read by both stages. Six snapshots that had given a compiled
`Map` or `Chain` such a JS iterator go back to the hand-written form.

Measured: 23 corpus snapshots move, each as native prints, 8,797 lines to
8,991 (×1.022), with two new cases,
`let_held_maps_given_a_compiled_chain_and_zip` and
`zip_of_sides_only_callers_know`. The React port's JS
regenerates byte for byte. `@rust-js/std` is the same size, 1,307,950
bytes. In 10,000 lines, `$iter` goes from 83.9 to 84.0 and renamed `x$1`
from 468.7 to 470.5 (std's `Zip` binds more of its own locals), while
role fallbacks go from 256.7 to 256.6. Mutations `zip-not-flipped`,
`flipped-given-a-let-made-in-place`, `flipped-given-a-call-made-in-place`
and `zip-of-callers-sides-flipped` (`lower/body_queries/in_place.ts`) are
caught.

## Amendment: the flatten family flipped (2026-11-16, #1681's change 2)

A `flatten()` of a chain the predicate doesn't admit is std's compiled
`Flatten`, `HELD_FLIPPED`'s, no longer the recognizers' `flatMap` there.

Measured: 8 corpus snapshots move, each as native prints, 2,868 lines to
3,482 (×1.214): std's `Flatten` is given its source's step, its items'
`IntoIterator` dictionary and its drops where `flatMap((item) => item)`
was one call. The React port's JS regenerates byte for byte. `@rust-js/std`
is the same size, 1,307,863 bytes. In 10,000 lines, `$iter` goes from
84.0 to 83.9, `Iterator.from` from 8.2 to 8.1, renamed `x$1` from 470.3
to 470.4 and role fallbacks from 256.7 to 256.5. Mutation
`flatten-not-flipped` (`lower/body_queries/in_place.ts`) is caught.

## Amendment: the peekable family flipped (2026-11-16, #1681's change 2)

A `peekable()` of a chain the predicate doesn't admit is flipped,
`HELD_FLIPPED`'s. std's `Peekable` was std's compiled struct there
already, so what changes is what's past it, flipped from the source
outward: an `enumerate()` of one is std's compiled `Enumerate`, not the
recognizers' `$enumerate` of a compiled `Peekable`.

Measured: 2 corpus snapshots move, each as native prints, 492 lines to
513 (×1.043). The React port's JS regenerates byte for byte, and
`@rust-js/std` is the same size, 1,307,863 bytes. In 10,000 lines,
renamed `x$1` goes from 470.1 to 470.3 and role fallbacks from 256.5 to
256.4. Mutation `peekable-not-flipped` (`lower/body_queries/in_place.ts`)
is caught.

## Amendment: the rev family flipped, and why the cycle is gone (2026-11-16, #1681's change 2)

A `rev()` of a chain the predicate doesn't admit is std's compiled `Rev`,
`HELD_FLIPPED`'s, no longer the recognizers' `$rev` of a source made
knowing its back (`$knowing`, `$mapEnds`). The amendment that flipped
`chain` left `rev` out. `back_stepped`, which the flip reads, asks whether
a `rev()` is made in place, and flipping one would change what decided it.
The architecture rule "`HELD_FLIPPED` never contains `rev`" held that.

The zip amendment's rule ends the cycle. `back_stepped` reads the ground
alone (`ground_made_at`), and before the flip, which may then take a
`rev()` out. But nothing the flip makes is given what the ground made: a
call made in place, or a `let` of one (`held_links`). What `back_stepped`
marked of an in-place `rev()` is a JS iterator, made knowing its back,
and that blocks the flip of whatever it's given to. What the flip takes
out is a `rev()` whose source is std's struct, which `back_stepped`'s
marks never reached. So the flip never changes what decided it. The
architecture rule is now the order (`back_stepped` before `decide_held`),
`back_stepped`'s read of the ground alone, and `held_links`' check of
what each link is given.

Its row: a `let` of a `rev()` read once, by a zip, a map and a chain the
predicate doesn't admit (corpus `let_of_an_in_place_rev_given_to_held_chains`).
The `rev()` is made in place, and each chain on it stays the hand form.
Mutation `flipped-given-anything-in-place` drops the check of what a link
is given; it gives std's compiled `Zip`, `Map` and `Chain` that JS
iterator, and the row catches it. The row runs against native in both
builds, the plain one and the build of std's structs.

Measured: 29 corpus snapshots move, each as native prints, 9,177 lines to
10,114 (×1.102). The matrix's 38,217 rows and the conformance table's
2,762 that the change names all pass. The React port's JS regenerates
byte for byte. `@rust-js/std` is the same size, 1,307,863 bytes. In
10,000 lines, `$iter` goes from 83.9 to 83.8, renamed `x$1` from 470.3 to
469.0 and role fallbacks from 256.4 to 255.6. Mutation `rev-not-flipped`
is caught.

## Amendment: the flat_map family flipped (2026-11-16, #1681's change 2)

A `flat_map()` of a chain the predicate doesn't admit is std's compiled
`FlatMap`, `HELD_FLIPPED`'s, no longer the recognizers' `$flatMap` there.
With it the heavier six families are all flipped: `chain`, `zip`,
`flatten`, `peekable`, `rev` and `flat_map`.

Measured: 10 corpus snapshots move, each as native prints, 4,769 lines to
4,923 (×1.032). The React port's JS regenerates byte for byte, and
`@rust-js/std` is the same size, 1,307,863 bytes. In 10,000 lines,
renamed `x$1` goes from 469.2 to 469.1 and role fallbacks from 255.6 to
255.5. Mutation `flat_map-not-flipped` (`lower/body_queries/in_place.ts`)
is caught.

## Amendment: the light families flipped (2026-11-16, #1681's change 2)

After the heavier six, each light family is flipped in its own commit, an
entry of `HELD_FLIPPED`: a chain the predicate doesn't admit makes it
std's compiled struct, not the recognizers' hand-written form. `fuse` is
none of them, as std's `Fuse` is its compiled struct wherever it's made.
Each row below is measured as the heavier families were: the corpus
snapshots it moves, each as native prints, the React port's JS (byte for
byte in each), `@rust-js/std`'s size, and a mutation that takes its entry
out, caught.

| Family | Snapshots | Lines | `@rust-js/std` | Readability, in 10,000 lines | Mutation |
|---|---|---|---|---|---|
| `inspect` | 7 | 2,737 to 2,844 (×1.039) | 1,307,863 bytes, the same | renamed `x$1` 469.0 to 468.8, `Iterator.from` 8.1 to 8.0 | `inspect-not-flipped` |
| `skip` | 53 | 17,433 to 17,622 (×1.011) | 1,307,863 bytes, the same | renamed `x$1` 468.7 to 470.3, role fallbacks 255.3 to 255.7, `$iter` 83.7 to 83.6 | `skip-not-flipped` |
| `take` | 56 | 16,761 to 17,227 (×1.028); two cases' library builds now run as their programs, `library-differs` taken off | 1,307,863 bytes, the same | renamed `x$1` 470.1 to 470.5, role fallbacks 255.6 to 255.5, `$iter` 83.6 to 83.5 | `take-not-flipped` |
| `step_by` | 29 | 9,975 to 10,329 (×1.035) | the same: std's `StepBy` was compiled already | renamed `x$1` 470.1 to 469.6 | `step_by-not-flipped` |
| `filter` | 73 | 18,986 to 20,004 (×1.054); `filter` taken off `MEANING_ONLY` too, which #1973 held back; a filter made in place counted is `.filter(f).length` still, its `count` std's only where it's std's struct; three shapes refused before run as native Rust, `filter_shapes_once_refused.rs`, a `rev()` of a filter of a `skip` its hint asked (#1427), a `peekable()` filter's hint (#382) and a filter cloned where its source isn't bound by `Clone`, taken off the guide's limitations | 1,306,986 bytes, the same: std's `Filter` was compiled for the meaning build already | renamed `x$1` 470.1 to 469.4, role fallbacks 255.3 to 254.6, `$iter` 83.7 to 83.4, `Iterator.from` 10.1 to 10.3 | `filter-not-flipped` |
| `filter_map` | 10 | 1,861 to 2,068 (×1.111): a held one's compiled `next_back`, `rfold` and `try_rfold` given as its dictionary where a `map` of it steps it from its back; `filter_map` taken off `MEANING_ONLY` too, which moves 5 of the 10 (`filter_map-meaning-only`); three shapes refused before run as native Rust, `filter_map_shapes_once_refused.rs`, the hint of one of a type parameter's, of a `rev()` of one of a `skip` (#1427) and of a `peekable()` one (#382), their diagnostics rows gone, the guide's limitation re-pointed at `cloned()` and `copied()` (#1998) | 1,310,068 bytes, the same: std's `FilterMap` was compiled for the meaning build already | renamed `x$1` 467.5 to 467.3, role fallbacks 253.4 to 253.3, `$iter` 84.3, the same | `filter_map-not-flipped`, `filter_map-meaning-only` |
| `take_while` | 9 | 2,571 to 2,593 (×1.009); `take_while` taken off `MEANING_ONLY` too, which moves 3 of the 9 (`take_while-meaning-only`); `held-function-unsupported`'s row moved to a held `map_while`, which the arm still serves | 1,310,475 bytes, the same: std's `TakeWhile` was compiled for the meaning build already | renamed `x$1` +5 and role fallbacks +2 in the moved files, 467.3 and 253.3, the same to a tenth; `$iter` the same | `take_while-not-flipped`, `take_while-meaning-only` |
| `skip_while` | 8 | 2,922 to 2,950 (×1.010); `skip_while` taken off `MEANING_ONLY` too, which moves 2 of the 8 (`skip_while-meaning-only`) | 1,310,967 bytes, the same: std's `SkipWhile` was compiled for the meaning build already | the same | `skip_while-not-flipped`, `skip_while-meaning-only` |
| `map_while` | 6 | 1,754 to 1,744 (×0.994); `map_while` taken off `MEANING_ONLY` too, which moves 2 of the 6 (`map_while-meaning-only`); drops/types.rs's held-function arm retired, each type it named std's compiled struct now, which the arm before it takes (ADR 0093) | 1,310,967 bytes, the same: std's `MapWhile` was compiled for the meaning build already | renamed `x$1` 467.1 to 466.9, role fallbacks 253.5 to 253.2, `$iter` 84.5 to 84.3 | `map_while-not-flipped`, `map_while-meaning-only` |
