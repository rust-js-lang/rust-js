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
agree with native Rust or are refused. (Those probes were too few: programs
later found three that ordinary Rust reaches, the amendment below.)

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

## Amendment: a known type's call that gives std's dictionary a `&mut` is never the trait call's (2026-11-12, #1487)

"No build is wrong by it" was wrong. 195 one-probe programs of the 21
functions, each called on every receiver std implements it for and compared
with native Rust, found three that ordinary Rust reaches with no flag:
#1486 (`as_ref()` through a `&mut` to text gave its cell), #1487
(`m.borrow_mut()` of std's `BorrowMut<T> for T` read `m.value` and threw)
and #1488 (`h.finish()` of an `h: &mut DefaultHasher` threw). The net says
which functions have a wrong fallback; only programs say whether one is
reached.

#1487's was the mechanism itself. A std dictionary's entry is written for
generic code, where a `&mut` is a box, `{ value }` (`MutRefForm`'s
`GenericBox`); a type known where it's called gives a `&mut` to an object
as the object. The trait call's arm in `mir/dispatch.rs` kept a place's
`&mut`, a number's, from std's dictionary (`gives_ref`), and let an
object's through. Now `gives_a_mut` (`mir/assign/boxed_args.rs`) says it
of the arguments' types too: a call whose receiver is no `dyn`, given any
`&mut`, isn't std's dictionary's. It's its recogniser's, or refused ("does
not support calling .."). A `dyn`'s dictionary is its own, a type
parameter's its caller's, and an impl rust-js compiled, the crate's or a
library's, takes the crate's forms, as before: `f.write_str(..)` of a
library's own `impl fmt::Write` is its dictionary's still, which
`test/crates.test.ts`'s `library_own_methods_through_its_dictionary` holds.

A first cut asked `read_only_generically` instead, every trait whose
dictionary only generic code reads. The crate corpus refused it: `a * b`
of known types in rust_decimal and chrono goes by an operator's
dictionary, rightly, as a value's form is the same in both. The form that
differs is the `&mut`'s, so the rule is of it.

Over the corpus's 2,307 cases no call took that arm with a `&mut`, so no
snapshot changes; what changes is what a missed shape does.
`unrecognizedDiffers` shrinks from 20 functions to 4: `Hasher`'s six
writes, `BorrowMut::borrow_mut`, `AsMut::as_mut`, and `Read`'s, `BufRead`'s
and `Seek`'s eight are refused without their recognisers. The 4 left take
no `&mut` of a known type: `Borrow::borrow`, `AsRef::as_ref` and
`IntoIterator::into_iter`, and `Debug::fmt` of a type parameter (#1477).

`test/tidy-stages.test.ts` holds a `Hasher`'s, a `BorrowMut`'s and a
reader's refused without their recognisers. Mutations
`known-receivers-mut-given-to-stds-dictionary`,
`dyns-mut-kept-from-its-own-dictionary` and
`librarys-mut-kept-from-its-own-dictionary`
(`lower/mir/assign/boxed_args.ts`). The `dyn`'s went on 2026-11-13: its
caller's `known_by_a_dictionary` of the `dyn` admits the call too, and no
JS changed without it (ADR 1004's last amendment).

## Amendment: a formatting trait's `fmt` is never the trait call's, and `Borrow::borrow` is right without its recogniser (2026-11-12, #1477)

A formatting trait's `fmt`, `Display`'s, `Debug`'s, `LowerHex`'s and the
others', of an impl rust-js didn't compile, is never routed through the
trait call: its dictionary's entry takes the value and gives its text, with
no `Formatter`, so without its recogniser `fmt_to_held_formatter.rs` wrote
into the held formatter's place what it gave back. One condition beside
#1487's (`is_fmt_method`, src/lower/recognition/fmt.rs) sends what no
recogniser takes to the refusal; a crate's own impl is its trait call as
before. `std::fmt::Debug::fmt` leaves `unrecognizedDiffers`; a tidy-stages
test holds the refusal, mutation fmt-through-the-trait-call. And
`Borrow::borrow` leaves it too: without its recogniser, each of the 14
corpus cases that call it runs as native Rust does or is refused (agent-fable's
sweep of 16 pairs, rerun here). `AsRef::as_ref` leaves with #1518, and
`IntoIterator::into_iter` with its adaptation by the parameter.

And `IntoIterator::into_iter` leaves it: what generic code bounded by
`IntoIterator` steps is adapted by what it's given, its type, not by the
recogniser's answer. A `&mut` to a sequence, an array, a slice or a
`Vec`-like, is a handle on each item, `$mutItems(v)`, where no recogniser
answers `into_iter` of it (`ref_into_iter`); a `&mut Result`, `iter_mut()`'s
handle, never its shared `iter()`'s value (`iterable_items`). Without the
recogniser, `iterables_given_to_generics` gave the array itself, whose
items generic code wrote as boxes, and `result_given_as_items` lost the
write through its item. A tidy-stages test holds both; mutations
mut-sequence-adapted-by-its-recogniser and mut-result-given-its-shared-items.
`AsRef::as_ref` is the list's one left, under #1518.

And `AsRef::as_ref` leaves it, after #1518's fix (84c533ca2): without its
recogniser each of the 143 corpus cases that call it runs as native Rust
does or is refused. The list is empty.
