# 0908. One meaning per iterator, checked by a build of std's structs alone

Status: Accepted, under the user's goal of 2026-10-26 ("keep going until
all done, the choice is your recommendation"). The first phase of the
plan "rust-js: one representation per iterator". Extends
[0871](0871-rev-is-stds.md) and [0561](0561-std-bodies.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): none for programs; this
is a test hook and a test.

## Context

A root-cause reading of the 188 wrong-answer fixes among 338 commits since
2026-09-25 found 37% were std's behavior reproduced wrong and 14% one
iterator in two forms mixed up. That's about 56% about iterators. Since
ADR 0871 an adapter std compiles is std's struct where it's held and the
recognizers' JS chain where it's made in place, and code that mixes the
two forms breaks. The plan makes std's struct every adapter's one
meaning, with the readable chains as rewrites a rule allows. Its first
step is a way to see what that meaning does alone.

## Decision

- **`--std-iterators`, a test hook**: no adapter is made in place (the
  in-place sets `collect_bodies` makes are emptied), so each adapter std
  compiles is std's struct. It's no way to build, nor an option a
  project sets: the test below is its one user, as `--tidy-until` is
  tidy's (ADR 0659).
- **`test/iterator-forms.test.ts`** compiles each corpus case with it and
  runs it against native Rust: a few cases in the suite, every case each
  night on two machines. The cases it doesn't match yet are listed in
  `differs`, each with why. One that comes to match fails the test until
  it's taken off, so the list only shrinks, and its length is the
  measure of the plan's progress.

## Consequences

- At ba033610c, 33 of the corpus's cases differ:
  - 16 are refused, most because std's `Rev` asks the back of an adapter
    std doesn't compile yet (`Skip`, `Take`, `StepBy`, `Chain`,
    `Copied`, `Zip`);
  - 2 crash rust-js;
  - 15 run otherwise, where std's struct steps or clones what the
    recognizers made (`it.back is not a function`, `f[$cloneKey]`), or
    print otherwise.
- No program's JS changes.
