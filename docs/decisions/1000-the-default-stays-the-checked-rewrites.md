# 1000. The default build stays the recognizers' rewrites, checked against std's meaning

Status: Accepted, under the user's goal of 2026-10-26 ("keep going until
all done, the choice is your recommendation"). Closes the plan "rust-js:
one representation per iterator" ([0908](0908-one-meaning-per-iterator.md)):
its phase 4, flipping the default to std's structs, is decided against.

Case ([0262](0262-when-rust-and-js-disagree.md)): none for programs. It
changes which build is the default, which stays as it is.

## Context

The plan made std's compiled struct every adapter's one meaning (phase
2), with the recognizers' readable JS as rewrites of it (phase 3). Its
phase 4 was to flip the default build to std's structs once they read as
well: "readability parity".

Measured with the scratchpad's `gap.ts`, the `--std-iterators` build was
longer than the normal one at each step: 11.7%, 24.8% after `Zip`, 9.9%,
12.9%, 12.0% after [0995](0995-a-std-body-takes-a-js-iterators-dictionary-by-default.md),
and 12.1% now. What's left is the representation itself. A std struct is
called with its dictionaries, `revExactSizeIterator_len(x, ..)`, where the
recognizers write `$iter(v).map(f)` or `v.length`. Parity would mean the
std build writing the recognizers' forms where adapters are made in place,
which is what the normal build already is. Flipping would make every
program's JS longer for no change in what it does.

What phase 4 was for is had otherwise:
- **One meaning:** `test/iterator-forms.test.ts` runs every corpus case
  with `--std-iterators` against native Rust and the normal build. Its
  `differs` list is empty, and nightly runs the whole corpus.
- **Std's struct where the rewrite can't be right:** each such form is
  std's struct by a rule, a held adapter (ADR 0871) or a zip that folds
  apart ([0997](0997-a-zip-that-folds-apart-is-stds-struct.md)), which
  #380 waited on the flip for and didn't need.

## Decision

The default build stays: recognizers' rewrites where an adapter is made
in place, std's struct where it's held or where a rule says the rewrite
can't be told apart from std's meaning. `--std-iterators` stays the test
hook that checks the rewrites against that meaning. The plan's phase 4
is not done, by this decision. A difference the forms test finds is fixed
in the rewrite, or by a rule making that form std's struct.

## Consequences

The open items the plan carried go on as their own issues:
- #386: generic code's `eq()`;
- generic code's zip, the guide's difference 15;
- #385: the refusals;
- #383: `next_back()` of an in-place `enumerate()` of a `fuse()`.

None waits on a flip.
