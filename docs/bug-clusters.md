# Bug clusters and the refactors that end them

One bug is fixed where it is. Many bugs of one mechanism in one owner say
that the owner's structure keeps making them, and the fix that ends them is
a refactor: one owner, a carried fact, a catalog, an oracle. This page is
the practice of finding those clusters in the history of fixes, and the
record of which clusters a structural fix ended.
[bug-kinds.md](bug-kinds.md) is the authority on kinds and areas;
[finding-wrong-answers.md](finding-wrong-answers.md) on the nets that find
bugs. This page is about **which refactor to do next, and whether the last
one worked**.

## The practice

Audit weekly, and before choosing the next refactor. The issue count in
[bug-kinds.md's "Reading the signal"](bug-kinds.md#reading-the-signal) sees
only bugs that had an issue. Most fixes never had one: of the 1,965
bug-fixing commits to 2026-11-07, 570 name the issue they fix. So the audit
reads the commits too.

1. **Collect the fixes.** Every `fix(..)` commit, and every `feat` or
   `refactor` whose message says it fixes a wrong answer, a P0 or a
   regression. For each, its date, what it fixed (wrong answer, crash,
   refusal or readability: the priorities' four), its kind and area as
   bug-kinds.md asks them of what the fix changed, and whether it was a
   regression and of what commit. Take the kind and area from the fixed
   issue's labels, or the message's own `Kind:` and `Area:` lines, before
   judging them yourself.
2. **Count.** Kind × area by week, and the owners most often changed by
   fixes of one kind. A cell, or a mechanism within one, of three or more
   fixes a week is a cluster.
3. **Name each cluster's mechanism** in one line: what the fixes had in
   common, and the files they changed.
4. **For each cluster, find whether a structural fix landed**, in the
   commits and ADRs, and compare the cluster's rate before and after it.
5. **For each cluster still active with none, propose the refactor** its
   kind calls for (bug-kinds.md's "A cluster calls for"), with its evidence:
   commits, issues and owner files. File it as a `Refactor:` or `Test net:`
   issue, and add its row below.

A fix with no issue says its kind, area and regression in its message, as
`Kind: missing case. Area: iterators. Not a regression.`, so the next audit
reads them instead of judging them again.

### The commands

```sh
# The week's fixes, each with the source files it changed
git log origin/main --no-merges --since='7 days ago' --date=short \
  --format='@%h %ad %s' --name-only \
  | awk '/^@/ { keep = ($3 ~ /^fix/) || /wrong answer|P0|regression/; if (keep) print; next }
         keep && /^(src|std|runtime)\//'

# The owners the week's fixes changed most
git log origin/main --no-merges --since='7 days ago' --format='@%s' --name-only \
  | awk '/^@/ { keep = /^@fix/; next } keep && /^src\//' | sort | uniq -c | sort -rn | head -20

# One cluster's fixes per day: its owner's files and its mechanism's words
git log origin/main --no-merges --since='21 days ago' --date=short --format='%ad %s' \
  -- src/to_oxc.rs src/to_oxc/ | grep -E '^[0-9-]+ fix' | grep -iE 'comment|blank line' \
  | cut -c1-10 | uniq -c

# The cluster's issues filed since its structural fix: new finds, not backlog
gh issue list --state all --limit 500 --json number,title,createdAt \
  --search 'label:"area: mut-refs" label:"P0: wrong answer" created:>=2026-11-04'
```

### Judging "eliminated"

The structural fix's date is the commit that made one owner the only
decider, or that landed the oracle. Compare two windows:

- **Before**: the cluster's fixes per week in the 14 days before it.
- **After**: its fixes per week since, and above all the issues of the
  cluster *filed* since. A fix after the refactor often closes an issue
  found before it; an issue filed after it is the structure failing.

| Status | When |
|---|---|
| **Eliminated** | No new issue of the cluster for 3 days or more after the fix, while the nets that found the cluster kept running, and every fix since closed an issue filed before it. Recheck it after a week. |
| **Shrinking** | Fixes per week after are half those before or fewer, and new issues still come. |
| **Still active** | The rate hasn't halved. Its fixes patched instances, not the structure. |
| **Too early** | Fewer than 3 days since the fix. |
| **Not yet fixed structurally** | No structural fix has landed. It has a proposed refactor below. |

A new net raises a cluster's rate before it lowers it: the iterator matrix
(ADR 1090) and the conformance table (ADR 1092) found old gaps in bulk on
2026-11-04 to 11-06. Judge such a cluster by what the net finds after its
first full run, not by the spike.

## The audit of 2026-11-07

**Scope.** 4,958 commits from 2026-09-25 to 2026-11-07 (5,152 with
merges). **1,965 fix bugs**: 1,929 `fix` commits, 27 `feat` and 9
`refactor`. By outcome: 1,005 wrong answers, 624 readability, 173 refusals,
163 crashes or blockers. 106 are fixes of regressions: 44 caused by a
correctness fix, 37 by a readability change, 11 by a feature, 3 by a
refactor or other change, 11 not named.

**How they were judged.** 585 took their kind and area from their issue's
labels or their own `Kind:` and `Area:` lines. The other 1,380 were judged
from their subject and the files they changed. Checked against the 566
fixes whose issue has a priority, the judgement agrees on the outcome for 89%
of wrong answers (74% of those it calls wrong answers are), on the area for
77%, and on the kind for only 55%. Kind is the hard one: a subject says
what changed in the output, not what the fix changed in the compiler. So
the clusters below are drawn by mechanism, a subject's words and an
owner's files, and checked against the issues' labels, not taken from the
kind × area count alone.

### Kind × area of the fixes

Wrong answers only (1,005), cells of 10 or more:

| Area × kind | Fixes |
|---|---:|
| `iterators` × `missing case` | 187 |
| `iterators` × `protocol gap` | 105 |
| `readability-passes` × `unsound optimization` | 103 |
| `functions-calls` × `missing case` | 96 |
| `classes-js-objects` × `missing case` | 65 |
| `mut-refs` × `missing case` | 64 |
| `ranges-numbers` × `missing case` | 59 |
| `modules-linking` × `missing case` | 54 |
| `drops-unwind` × `missing case` | 53 |
| `nullability` × `missing case` | 19 |
| `iterators` × `duplicated authority` | 16 |
| `functions-calls` × `duplicated authority` | 14 |
| `mut-refs` × `duplicated authority` | 12 |
| `declarations` × `missing case` | 11 |
| `iterators` × `reconstructed fact` | 10 |

Fixes of each area by week (Monday's date), all fixes / wrong answers:

| Area | 09-21 | 09-28 | 10-05 | 10-12 | 10-19 | 10-26 | 11-02 |
|---|---|---|---|---|---|---|---|
| `iterators` | 3/1 | 32/28 | 48/40 | 47/38 | 84/72 | 93/64 | 131/93 |
| `readability-passes` | 0/0 | 1/0 | 14/11 | 38/26 | 81/38 | 239/33 | 147/8 |
| `functions-calls` | 4/3 | 12/9 | 14/13 | 18/15 | 48/42 | 83/26 | 41/10 |
| `mut-refs` | 1/1 | 1/0 | 14/14 | 12/8 | 30/28 | 53/15 | 43/19 |
| `classes-js-objects` | 1/0 | 4/2 | 46/40 | 14/10 | 16/13 | 63/16 | 30/7 |
| `modules-linking` | 2/2 | 12/9 | 18/15 | 20/13 | 15/12 | 35/10 | 45/10 |
| `ranges-numbers` | 1/1 | 10/8 | 15/14 | 7/7 | 30/25 | 38/14 | 16/4 |
| `drops-unwind` | 0/0 | 5/5 | 6/6 | 13/12 | 19/12 | 21/14 | 36/21 |
| `nullability` | 0/0 | 1/1 | 2/2 | 4/2 | 11/9 | 16/7 | 25/11 |
| `declarations` | 0/0 | 0/0 | 7/4 | 3/2 | 4/2 | 6/3 | 16/5 |

The owners most changed by fixes of the largest cells:

| Cell | Files most changed (fixes) |
|---|---|
| `readability-passes` × `missing case` | `mir/tidy/stages.rs` 91, `src/to_oxc.rs` 52, `mir/tidy.rs` 43, `mir/tidy/jumps/tails.rs` 34, `mir/tidy/branches.rs` 31 |
| `iterators` × `missing case` | `mir/iter.rs` 44, the runtime 37, `iterators.rs` 33, `iterators/knowing.rs` 25, `combinators.rs` 22 |
| `iterators` × `protocol gap` | the runtime 90, `runtime/back_steps.js` 26, `runtime/closing.js` 26, `recognition/iterators.rs` 26, `mir/iter.rs` 24 |
| `readability-passes` × `unsound optimization` | `mir/tidy.rs` 28, `mir/tidy/stages.rs` 20, `mir.rs` 16, `mir/tidy/branches.rs` 13, `mir/tidy/logicals.rs` 11 |
| `mut-refs` × `missing case` | `mir/assign.rs` 15, `mir.rs` 14, `analysis/cell_borrows.rs` 14, `std_types/cell.rs` 12, `drops/ref_cells.rs` 10 |
| `functions-calls` × `missing case` | `mir/calls.rs` 23, `traits.rs` 22, `calls.rs` 18, `std_impls.rs` 15 |
| `drops-unwind` × `missing case` | `drops.rs` 15, the runtime 12, `drops/inlined.rs` 10, `mir/drops.rs` 9, `drops/parts.rs` 7 |

### The results

Fixes per week before are over the 14 days before the structural fix;
after, since it, to 2026-11-07. "New issues" are those of the cluster filed
after it.

| Cluster: mechanism | Fixes (wrong answers) | Structural fix | Before /wk | After /wk | New issues | Status |
|---|---:|---|---:|---:|---:|---|
| A readability rewrite moves, repeats or drops a read or a drop (`readability-passes` × `unsound optimization`) | 116 (116) | Tidy's stage differential, c4d2fa538 (10-25), didn't stop it: 31.5 a week to 22.1. ADR 1087's on/off oracle (2f4accd34) and ADR 1088's `js::Writes` (2bda1585a), 11-04 | 37.5 | 4.7, 2 fixes in 3 days: #779 and #756, neither an unsound rewrite | 0 | **Eliminated** |
| A readability change regresses what worked | 37 regression fixes | ADR 1087, as above | 9 on 11-02, 5 on 11-03, 8 on 11-04 | 1 on 11-05, none since | 0 | **Eliminated** |
| A `.d.ts` that says other than the JS does (`declarations` × `duplicated authority`) | 36 (16) | ADR 1091's `JsSignature` (bc4596c04), and generated TypeScript callers (15e262843), 11-04 | 6.5 | 0 | 1, #842: a representation gap, not two deciders | **Eliminated** |
| One `&mut` with several JS forms (`mut-refs`, wrong answers) | 85 (85) | ADR 1086's `MutRefForm` (b96990796) and the `&mut` matrix (c84b56797), 11-04 | 25.0 | 2.3 | 1, #786 | **Shrinking**; step 3 is #821 |
| A module's loads and specifiers (`modules-linking`) | 66 (22) | Loads symbolic until output planning, relinked: e0d5b7077, 44b3e851f, 11-04 | 20.5 | 2.3 | 0 of loads; 7 of the area were dev-loop | **Shrinking** |
| An adapter doesn't pass a step on (`iterators` × `protocol gap`, no panic) | 85 (79) | ADR 1089's `$forward` (5985f5dbe) and `OwnStep` catalog (0e82370f6), 11-04; ADR 1092's contract and conformance table (0ed9220b8, 85b1ad006), 11-05; `$owns` (331296c3b), 11-07 | 33.0 | 25.7 after 11-04, 17.5 after 11-05 | 14 after 11-04, mostly the new table's finds | **Still active**, draining: protocol-gap issues filed 16, 16 and 11 a day from 11-04, 1 on 11-07 so far |
| Integers, floats, chars and ranges (`ranges-numbers`, wrong answers) | 60 (60) | Boundary matrices, 22d1e8a8a (10-05) | 7.9 | 10.6 over the next month; 4 in the week of 11-02 | 1 wrong answer since 11-03, #657 | **Shrinking**, by coverage rather than one refactor |
| Calls, closures and dispatch (`functions-calls`, wrong answers) | 95 (95) | None single; the evidence list (ADR 0886) in part | 37 the week of 10-19 | 20, then 9 a week | | **Shrinking** |
| A trait default or generic evidence in the wrong terms (`functions-calls` × `missing case`) | 47 (23) | ADR 0886's one evidence list, c9c8bd348 (10-25), didn't stop it: 6.5 a week to 16.2. `traits::default_args` (d58234973) and `impl_type` gone (aa856b74a), 11-06 | 15.5 | 0 in 1 day | 0 | **Too early** |
| What an iterator drops as a panic unwinds (`iterators`/`drops-unwind`, panics) | 65 (65) | #817: the panic points table (4cf0e148d) and `$unwinds` (a8fec5989), 11-06 | 28.5 | 2 in 1 day | 1 | **Too early**; #860 and #862 open |
| Stepped from its back by a per-type fact (`iterators` × `reconstructed fact`) | 100 (68) | #730's one `IterValue` (0858c43af), 11-05: rose to 63 a week, the matrix's finds. #814 and #851's per-value facts (63bf1a6bd, 0d95dc5f4), 11-06 | 39.0 | 0 in 1 day | 0 | **Too early**; #851 open, 15 mutations survive there (#863) |
| When a collect asks `size_hint` | 43 (38) | ADR 1097's `hint_asks` (096ea98f6), 11-06 | 12.0 | 0 in 1 day | 0 | **Too early**; #820 and #858 open |
| An iterator closed twice, or dropped again (lifecycle) | 61 (61) | #870's `$state` (c248fab04, 8e100bb79, c2a604658), 11-07 | 25.5 | | | **Too early**; #871 open |
| A decision that depends on a map's walk order | 2 fixes, 519 walks | #866: ordered maps (51d79d71d, ADR 1098) and the determinism net (b3416b692), 11-07 | | | | **Too early**; prevented by construction |
| Iterator fixes that break other iterator steps (regressions of a correctness fix) | 20 regression fixes | The per-push matrix slice (36d7f5184), 11-05; #818's gate, each change's rows run before it lands (e6ffee46a), 11-06 | 9 on 11-05 | 3 on 11-06, none on 11-07 | | **Too early** |
| The JS object model: getters, `this`, classes (`classes-js-objects`, wrong answers) | 88 (88) | `js::Known::Getter` (9d2f3df07, ADR 0672), 10-24 | 15.0 | 14.0; 7 in the week of 11-02 | | **Still active** at a low rate |
| A drop outside an iterator: drop glue, scopes, drop slots (`drops-unwind` × `missing case`) | 50 (50) | #887, 11-07: drop glue lowered from rustc's own `drop_glue` shim (`drops/glue.rs`), its parts, order, variants and unwinding rustc's, and what drops, how long and whether it may panic asking its parts; drop slots keyed by their type and numbered by `drop_params.rs` alone. Its first diff found #893 | 7 to 12 a week since 10-12 | | | **Too early**; scopes and `dyn FnOnce`'s drop not yet from MIR |
| What's made in turn leaks as a later part's making panics: a derived or std `Clone`'s or `Default`'s parts, a clone's items, `resize`'s, `fill`'s and `vec!`'s value, `inspect`'s (`drops-unwind` × `missing case`; the bodies rust-js writes itself, not from MIR) | #897, #898, #902, #903: 163 cases of #854's unwind table | ADR 1102, 11-08: one rule, `held_in_turn` in `std_impls/in_turn.rs`, asking `clone_may_panic` and `default_may_panic`; `$madeInTurn` and `$madeEach` in the runtime, each on `$unwinds`, as its own loops are | | | | **Too early**; `Vec::clone_from`'s order, #919, ended with the next row |
| A std method rust-js writes itself leaves its collection other than std's guard does as a drop or clone in it panics: a map's `clear`, a map's `extend`, `Option::insert`, `dedup_by`, a `Vec`'s `clone_from` (`drops-unwind` × `missing case`) | #899, #900, #904, #906, #919: 64 cases, 128 case-builds of #854's unwind table | ADR 1102 amended, 11-08: what std's method has written, emptied or taken stays so as a drop in it unwinds, asking `drop_may_panic` and `clone_over` (which knows a `Vec`'s `clone_from`, `$cloneIntoVec`); `$entryDropping` shared by `$extendMap` and `$mapFrom`; `$dedupByCells` one loop with `$dedupBy`, chosen by `MutRefForm` | | | | **Too early**; its probes found #920, #921, #922, filed |
| What std holds as a comparison of the crate's own `Ord` panics, and the order it asks in: a heap's sift, an unstable sort's gaps, a B-tree's search, `max`, `min` and `clamp` (`drops-unwind`/`functions-calls` × `missing case`; the runtime's ports of std's algorithms) | #901, #908, #911, #912: 294 cases, 588 case-builds of #854's unwind table pass (#911's `entry` and #924's `clone` rows left, #907's stable sort not begun) | ADR 0068, 0342, 0404 and 0057 amended, 11-08: one answer of whether comparing may panic, `order_may_panic` in `ordering.rs`, which every helper's unwinding drop asks; each sift and gap written back in a `finally`, as std's `Hole` and gap guards; a B-tree's one search per operation, `$found`, in std's argument order and one node's walk; `$maxOf`, `$minOf` and `$clampBy`'s drop | | | #924, #925, #926 | **Too early**; exact for a B-tree of up to 11 keys only, std's node layer (#926) and stable sort (#925) filed |
| Where a comment or blank line goes (readability) | 139 | None | 20, 57, 60 a week | | | **Not yet fixed structurally** |
| A temporary spilled or named after MIR (readability) | about 130 | None | 46, 48 a week | | | **Not yet fixed structurally** |
| A label or jump the Rust didn't write (readability) | 62 | None; #771 measures it | 37, then 14 a week | | | **Not yet fixed structurally**; #771: about 40% of generated functions still print one |

The first structural fix came before this practice: bodies lowered from
their MIR, not THIR ([ADR 0364](decisions/0364-mir-lowering.md), 10-13).
Its context lists the questions THIR left to rust-js, case by case, each an
ADR: the order operands run in, how long a temporary lives, what a reborrow
refers to, what's dropped as a panic unwinds. MIR answers each. The fixes
before it are too few to measure the change against.

**What the eliminated and shrinking clusters had in common.** Each fell
after one owner *and* an oracle that compares against native Rust landed
together: `js::Writes` beside the on/off differential, `JsSignature` beside
generated TypeScript callers, `MutRefForm` beside the `&mut` matrix. Tidy's
stage differential alone (10-25), and the evidence list alone (10-25),
didn't stop their clusters.

## Proposed refactors

For the clusters still active or not yet fixed structurally. Those already
filed are listed with their issue.

### Already filed

- **Finish `MutRefForm`'s operations exhaustive** (#821), with generated
  `MutRefForm` × operation and `NoneForm` × writer × reader matrices (#855).
- **The combinators' emitted unwinding onto `$unwinds`** (#860), and the
  std-shaped adapters' state as one explicit machine (#871): the panic and
  lifecycle clusters' remaining deciders.
- **Retire `back_stepped`'s per-type set** for maps, chains and the rest
  (#851), and the 15 mutations that survive there (#863).
- **One model of when a collect asks `size_hint`** (#820), whose refusals
  remain (#858).
- **Panic points beyond iterators** (#854) and **neighbour shuffle**
  (#853): the nets for the drops and context clusters.

### To file

1. **Drop glue lowered from rustc's own, not written by hand**: filed
   as #887 and landed on 11-07 (the drops row above). The `call_once`
   shim's `dyn FnOnce` drop stays rust-js's `$droppingOnce`, which the
   shim's MIR agrees with
   (`drops-unwind` × `missing case`, 50 wrong answers, 7 to 12 a week since
   10-12, with no structural fix). Three mechanisms recur: drop glue's parts
   as a part panics (f58ce2245 #865, 579539d03 #847, a1a886ee4 and
   46f5892fe #416), a `dyn FnOnce` or closure box's drop (781c6cbdc #409,
   a2f487c4d #464, 8284a6ffa), and a trait's associated type's drop slot in
   a copied default's terms (c19ebbaef, c57e555a3, 146866bb4 #798,
   272ab51ee). Lower rustc's `drop_in_place` and `call_once` shims from
   their MIR where they're reachable ([ADR 1085](decisions/1085-issues-carry-a-kind-and-an-area.md#what-the-first-count-calls-for)'s
   change 12), so the parts, their order and their unwinding are rustc's;
   and key drop slots by one derivation. Owners: `drops/parts.rs`,
   `drops/inlined.rs`, `drops.rs`, `mir/drops.rs`, `traits/drop_params.rs`,
   `traits/dyns.rs`. Net: #854.
2. **A comment's place carried from lowering, not found by spans as it's
   printed** (readability, 139 fixes, 60 in the last week; `src/to_oxc.rs`
   changed by 65 of them). Every fix teaches `comments_of`
   another gap: a `let` whose value opens (20b0739df #758 #767), a
   `continue` arm (508e85347 #693, d56517ab4 #680, 8c14c9c4d #666,
   2063a84e5 #610, 657096ec0 #574), a `catch_unwind` condition (1b6ef7e9c
   #688, 31a2f53a8 #605), a binding of no JS (176f79dbd #643, b2f3293df
   #634), an or-pattern's later arm (6a4af1c6c #608 #609, 20adba93b #595),
   a `use` (9ba9b4206 #642 #606). Each was filed a `missing case` of
   `comments_of`; together they are a `reconstructed fact`: where a comment
   goes is inferred from the source's gaps after tidy and preparation have
   moved the code. Attach each comment to the MIR
   statement or item it precedes as lowering makes it, carried as a fact
   that tidy's and preparation's moves take along, and printed where its
   node is; and generate each construct × comment position as a matrix.
   Owners: `src/to_oxc.rs` (`comments_of`), `src/to_oxc/trailing.rs`,
   `starts.rs`, `openings.rs`, `src/comments.rs`.
3. **Structured control flow from MIR in one pass, labels only where
   needed** (readability, 62 fixes; #771: about 40% of generated functions
   print a label the Rust didn't write). The fixes are each a rule in one
   of tidy's jump passes: 041f40768, 9aa895d5e, eb393a34c (#771, #791),
   79d6c42ce, 80cdbd4dc (#777), 9fe234511 (#523 #696), d1b0adad4,
   1c640687b (#583 #588), d44531c92 (#530). Replace the rules that remove
   labels after the fact with one structuring of the CFG that places each
   `if`, loop and `switch` by its dominators and leaves a label only where
   no structured form reaches the target, measured by #771's generated
   matches. Owners: `mir/tidy/jumps/`, `mir/tidy/stages.rs`,
   `mir/tidy/decisions/`, `mir/structure.rs`.
4. **One namer of temporaries, after what makes them** (readability, about
   130 fixes, 46 and 48 in the last two weeks; #837, #841 open). A
   temporary is spilled as `value` or named after MIR's `_12` where tidy
   didn't read it in place. Give naming one owner that takes each name from
   the call or binding that made the value (ADR 0569's rule), asked by
   every pass that spills, and a generated-program count of MIR-made names
   in the output as its measure. Owners: `analysis/naming.rs`,
   `mir/temporaries.rs`, `mir/tidy/bindings.rs`, `mir/tidy/stages.rs`.
5. **The JS object model as facts, with a JS-side oracle**
   (`classes-js-objects`, 88 wrong answers, still 7 in the last week).
   `Known::Getter` (10-24) made one hazard a fact; `__proto__` keys
   (5546ddd6f), a getter of a class's superclass (dfb97c4d6), a realm's DOM
   node (6ebab3068 #555) and `Cell::set`'s `this` (b1d0c4490 #359) were
   each found by hand. A counting-proxy oracle, each corpus case's exported
   values read through a `Proxy` that logs gets and sets, compared on and
   off, would find the rest, as ADR 1087's read oracle does for
   preparation. Owners: `classes/`, `untagged/`, `field_keys.rs`,
   `src/js/stmt_walk.rs`.
