# 1178. A block's `break`s lead on to what follows it

Status: Accepted (2026-11-21, the lead's go on lisa agent's design note for
#2215, steps 1 and 2). Amends [0629](0629-a-match-of-constants-is-a-switch.md)'s
label rule, D1, and [0754](0754-a-return-arms-go-on-to-is-in-each.md)
(step 3: a call's `return` is in each path).

Case ([0262](0262-when-rust-and-js-disagree.md)): none. The JS does what
it did. Only its control flow is written as a person writes it, without a
label the Rust doesn't have.

## Context

After 73b3f220c, std's JS still had 5 labels MIR made, listed in
`test/readability.test.ts` under #2215. That commit added tidy's
`breaks take their tail` (tidy/jumps/epilogues.rs). It writes each `break`
of a block as the tail that follows the block, where that tail is drops
that may throw and a `return`. The 5 left came in two shapes.

- **A continuation.** In `try_fold`, a `?`'s `Continue` breaks forward
  past the drops and `return` that its other ways go on to:
  `l: { if (c) { if (none) { .. } else { ..; if (Continue) { acc = a;
  break l; } r = ..; } dropF?.(f); return r; } acc = init; } <the fold of
  the rest>`. Std has three: Peekable's, SkipWhile's and StepBy's. What
  follows the block is the rest of the fold, too long to copy into a
  `break`. lisa agent's note proposed moving what follows an `if` whose
  `then` never goes on into its `else`. That alone takes none of the three
  away. The `if (none)` holding the `Continue` has a second way that goes
  on, so the drops and `return` after it still follow the `break`'s way.
- **A tail of `const`s.** Skip's `advance_by` ends `const value = n === 0
  ? undefined : n; return ..;` and Flatten's `size_hint` ends `const a =
  fhi; const b = bhi; return [lo, ..];`. Neither tail has a drop, so
  `breaks take their tail` left them.

## Decision

- **One tail rule.** `copied_tail` (epilogues.rs) answers which tails a
  `break` may be given, for both passes. A tail qualifies if it is a
  `return` after at most `SHARED` statements and either:
  - its drops are ones a `finally` may not hold, and each statement
    before the `return` is one tidy copies (`copied_once`); or
  - with no drop, it is one or more `const`s of what reads alike written
    twice.

  What reads alike is decisions.rs's `reads_alike_twice`, the purity
  question `copied_once` asks. Assignments with no drop are never such a
  tail. A lone jump is one since step 3 (below). At an earlier placement,
  copying a lone `return` made 349 snapshots worse.
- **`breaks lead on`** (tidy/jumps/continuations.rs) works on each
  MIR-made block, never one the program labeled. It goes through each list
  on a `break`'s way, the innermost first. Where an `if` holding a `break`
  is followed by `rest`, it applies one of two rules:
  - **Move.** If the `break`'s branch never goes on (`js::ends`), `rest`
    goes into the other branch, which then holds no `break` of the block.
    Nothing is copied.
  - **Copy.** If `rest` is a tail a `break` may be given, it is written at
    the end of each of the `if`'s ways that goes on.

  The move is refused where `rest` never goes on itself, `if (c) { break
  l; } s; return x;`, which is a guard as `branch_ends` leaves one (ADR
  0745): a `let .. else`, or a `switch`'s case. Both rules are refused
  where the receiving branch declares a name `rest` reads, or `rest`
  declares a name read before it. The block changes only if every `break`
  is last on its way then (`drop_tail`); otherwise it keeps its shape.
- **Run last.** Both passes run at the end of tidy's `finish` stage,
  before `written loop labels`, followed by `unlabeled` and by
  `unreachable_after`. The latter drops a tail that only the block's
  `break`s went on to, once the block's own end never goes on. There they
  see only the labels every other pass left. Placed at the end of `loops`,
  as `breaks take their tail` was, or of `chains`, they preempted later
  passes that write a match's tree, a flag's value and a `switch`'s guard
  better (`kept trees`, `flag values`): nine corpus snapshots got worse,
  then two.

## Consequences

- Std's JS has no label MIR made, and `stdLabels` is empty. Peekable,
  SkipWhile and StepBy's `try_fold` read `if (Continue) { acc = ..; } else
  { ..; dropF?.(f); return result; }`. Skip's `advance_by` and Flatten's
  `size_hint` write their tail where they `break`.
- No corpus snapshot moves. One generated match program, seed 11's `m13`,
  loses its label (`test/match-trees-labeled.txt`).
- Step 3: a lone jump, `continue;`, `break;` or `return x;`, is a tail a
  `break` may be given too. That is safe at the finish placement, which
  sees only the blocks every other pass left; the corpus has none, and no
  snapshot moves. `breaks take their tail` never writes a `break;` or a
  `continue;` of no label where a loop or a `switch` in the block holds
  the `break`, since it would leave that one instead (`looped`). This
  takes away 5 of the crate corpus's 21 labels: num-traits' f32 and f64
  `from_str_radix`, rust_decimal's `div_remainder32` and `div_remainder64`
  (whose loop's `break;` is now written at each way that goes on, its
  `// Do some scaling` back where the Rust writes it), and strsim's
  `generic_jaro`. The 16 left are listed in `listedLabels`, each with its
  reason in #2215. Each takes a shape these rules don't:
  - a block's `break` from inside a loop, `while (..) { if (!c) { break
    bb109; } .. }`, a loop exit that goes on past code (httparse's
    `parse_headers_iter_uninit`), or `continue bb65;` from an inner
    `while` (miniz_oxide's `init_tree`);
  - a tail longer than `SHARED`, its drops tested by liveness flags the
    `break`'s way has just set false (itertools' 11 `collect_from_iter`);
  - a tail of an assignment and a `continue`, which is no lone jump
    (rustls-pki-types' `validate`);
  - a match's merge broken to from deep inside an or-pattern's tree
    (uuid's `into_err`);
  - and itertools' probe `step_buffering`, a continuation `breaks lead
    on` refuses.
- Unit tests hold each guard. Mutations: `continuation-*` in
  `scripts/mutations/lower/mir/tidy/jumps/continuations.ts`, and
  `epilogue-consts-not-copied`, `epilogue-const-of-a-call-copied`,
  `epilogue-lone-jump-not-copied` and `epilogue-lone-jump-copied-into-a-loop`.
