# 0565. How the generated JS reads is a gate, as what it computes is

Status: Accepted. Extends [0050](0050-snapshots.md),
[0364](0364-mir-lowering.md) and [0314](0314-std-data-structures.md),
whose ratchet of what's known this is of how it reads.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. Nothing a program
can observe changes; this decides what the tests hold.

## Context

The north star is JavaScript people would have written by hand, and the
test suite holds the other half: the corpus and the snapshots say what the
compiler writes, held equal, and native Rust says what it computes. Whether
it reads well is read in a snapshot's diff, by whoever blesses it, and
nothing says afterwards whether the whole reads better or worse than it
did. ADR 0364 lowers from MIR and calls readability secondary, and the
inventory snapshot shows the cost that leaves: a `bb36:` label in a plain
`match`, a `for` over sorted tuples as `while (true) { const value =
$next(iter) ... }`, a `return;` where the function ends anyway. The one
measure of it, `bun run port measure`, counts seven such shapes in what a
compile of the react.dev port adds to its committed JS: a private port,
on one Mac, run by hand, a number nothing holds.

## Decision

**The shapes a person wouldn't write are counted over the generated JS
this repository commits, and no shape's share may grow.**
`scripts/readability.ts` is the one list of them, a test of one line each,
which `bun run port measure` reads too:

| Shape | The line |
|---|---|
| temporaries | `const _3 = ..`, a MIR local's name kept |
| labeled blocks | `bb36: {`, a basic block as a label |
| while (true) | a loop stepped by hand where a `for` was |
| $iter | an array stepped by position, `$iter(v)` |
| Iterator.from | a value made a JS iterator where it was one |
| return; | a bare return where the function ends |
| renamed x$1 | a name given a suffix for a clash the Rust hasn't |

`test/readability.test.ts` counts them over the corpus's and the
snapshots' JS, which their tests hold equal to what the compiler writes
now, and writes `docs/readability.txt`: each shape's count and its share,
in ten thousand lines to a tenth, so one more of a shape in a corpus this
size shows. A share that grew fails the suite, on the Mac and on CI, with
the construct to read; it's blessed (`bun run bless`, which blesses the
corpus and the snapshots too) only where the change's ADR says why. A
share that shrank is blessed as well, so the file says what's true, and
the diff shows what got better.

## Why

- **It's the measure that was, made a gate.** `port measure`'s seven
  shapes are what MIR adds that a person wouldn't write; counting them
  over committed JS asks the same question of what the suite already
  holds, with no port to clone and nothing private. The port's measure
  keeps running on the Mac, over the one list.
- **A share, not a count**, so a new corpus case that reads well lowers
  the number and one that brings a `bb` label raises it, whatever the
  corpus's size; a tenth, so a single labeled block in 85,000 lines is
  seen.
- **Both directions blessed**, as std coverage is (ADR 0314): a ratchet
  that only ever refuses would let the file drift from the truth, and a
  gain that nobody records is one nobody defends next time.
- **At 747 files and 86,064 lines**, the baseline is 0.1 temporaries, 2.4
  labeled blocks, 16.8 `while (true)`, 61.8 `$iter`, 14.3 `Iterator.from`,
  9.5 bare returns and 954.1 renames in ten thousand lines: the renames
  are the cost to read first.

## Alternatives

- **Gate the react.dev port on CI**: the port is private, and what it
  measures is the port's Rust against its hand-written original, which
  the repository has no copy of. Rejected for CI; kept on the Mac.
- **Hold the counts, not the shares**: a corpus that grows by a clean
  program would pass and one that grows by a dirty one would fail the
  same way, but a program with one `$iter` among two hundred lines would
  fail for making the JS read better. Rejected.
- **A readability score of the whole**, one number: hides which shape
  moved, and what to read. Rejected; the shapes are the list.
- **Refuse any growth without a bless**: that is this decision. Refuse
  growth even when blessed would stop a decision that needs a construct,
  `$iter` for a `peekable()` of an array (ADR 0561). Rejected.

## Consequences

- A change that adds a corpus case or snapshot runs `bun run bless`,
  which now blesses `docs/readability.txt` too; its diff is part of the
  review, as the snapshots' are.
- The renames' share, 954.1, is the first number to bring down: a name
  suffixed for a clash the Rust doesn't have, `value$1`, is a lowering
  that reads worse than it needs to.
- `port measure` prints its counts from the same list, so a shape added
  there is a shape the gate counts.

## What holds it

- `test/readability.test.ts`: no shape's share grew; the file is what the
  measure says; each shape is one line's, counted; the list is the seven.
- `docs/readability.txt`, the baseline, blessed with the snapshots.
