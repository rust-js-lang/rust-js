# 1128. What no recogniser takes is refused or right: each std function's recogniser left out in turn

Status: Accepted (2026-11-12, #1457, of the architecture review of that
day). Beside [1087](1087-each-readability-rewrite-is-checked-on-and-off.md)
and [1127](1127-a-fact-has-a-role-and-the-js-is-right-without-one-that-permits.md),
whose oracle leaves out a rewrite and a fact; and
[1124](1124-decision-coverage.md), which counts the answers the nets reach.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. Nothing a program
observes changes: two flags a build never passes, and tests. A build's JS
is the same byte for byte.

## Context

Of the 1,005 wrong answers the audit of 2026-11-07 counted, `missing
case` is the kind most often fixed: a recogniser, asked the right
question, doesn't match one shape of a call. What happens then is the
compiler's fallback. If the fallback is a refusal, a missing case is a
P2, safe; if it's JS, it's a wrong answer. Nothing measured which.

`Recognition::classify` is where a std function is taken by rust-js's own
model of it, asked at 85 sites, of lowering and of every analysis.

The experiment (2026-11-12): each of the corpus's 2,266 cases compiled
with each std function a recogniser took in it left out, in turn: 33,704
compiles of 2,113 std functions, each JS that changed run against native
Rust.

| What the compile without the recogniser came to | Compiles |
|---|---:|
| Refused at compile time | 28,181 |
| The same JS: another route lowers the call | 5,455 |
| Other JS, running as native Rust does | 11 |
| Other JS, throwing or printing otherwise | 57 |
| A crash of the compiler | 0 |

So for 2,092 of 2,113 std functions, what no recogniser takes is refused
or right. The 57 are 21 functions, each a std trait's method: `Hasher`'s,
`io::Read`'s, `BufRead`'s and `Seek`'s, `Borrow`, `BorrowMut`, `AsRef`,
`AsMut`, `IntoIterator::into_iter` and `fmt::Debug::fmt`. One mechanism:
the call falls to a route built for generic code, which hands the trait's
JS a receiver in generic code's form, or calls it as Rust declares it,
where the JS takes another shape (#1477). No build is wrong by it: probes
of each of those traits' methods over std's types, with their recognisers,
agree with native Rust or are refused.

## Decision

**A compile can leave a std function's recogniser out, and every program
compiled without each recogniser it used is refused, or runs as native
Rust does.**

- **One entry.** `classify` is `classified`, its body as it was, behind
  `unrecognized::left` (`src/lower/recognition/unrecognized.rs`): the one
  place a recogniser's answer can be withheld, so lowering and each
  analysis see the same.
- **`--without-recognizer <std path>`**, any number, names the functions
  by their std paths (`std::f64::<impl f64>::sqrt`); the session holds
  them (`session::Flags`). `--recognizers-found <file>` writes which
  functions a recogniser took, a line each (`session::recognized`). Only std's functions are
  named: a binding's has no std path, and stays. Of a build, neither is
  given, and `left` is two reads.
- **The net is ADR 1087's**, `test/tidy-stages.test.ts`: a `without the
  recogniser of <path>` variant of each function found in a program. A
  variant refused is what's wanted; one that compiles runs as native
  Rust does; one that crashes, or does otherwise, names the function.
  The suite's own run takes its few cases, the nightly the corpus and the
  generated programs.
- **What differs today is a list that only shrinks**,
  `unrecognizedDiffers`, the 21 functions of #1477, each with its issue:
  no variant is made of one. A name leaves it with the fix of its route.
- **A wrong answer the net finds is filed as its own issue**, naming the
  function; a P0 where a build is wrong by it.

## Consequences

- A recogniser's missing case, for the 2,092 functions the corpus
  reaches, is a refusal, held by a test. For the 21, it's #1477.
- A new fallback route that writes JS of an unrecognised std call shows
  as a variant that differs, on the night it lands.
- The net costs a compile of each (case, function) pair, most of them a
  refusal: about 34,000 of the corpus, 8 minutes on the 14-core Mac.
- It says nothing of what a recogniser does where it matches wrongly:
  that's the corpus's and the tables' to find, against native Rust.
- The 5,455 compiles that came out the same are calls another route
  lowers, `Iterator::map` by the iterators': their `classify` answer is
  read by an analysis, not by what writes the call.
- For #1458: the 11 that ran right by another route are where std's own
  body, or a dictionary's, already serves in a recogniser's place.
- Mutations: `recogniser-never-left-out` (`left` withholds none),
  `recognised-unrecorded`, `classify-past-what-is-left-out`.
