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
