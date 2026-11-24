# 1118. What development writes has one known place, each store bounded

Status: Accepted (2026-11-10, #1213)

## Context

The user's requirement: each machine is used only for rust-js development,
so how its disk grows must be well defined and well known. The M1 reached
the 40 GiB floor, where the loop's stores keep nothing and its tests fail
(#1208). The inventory (`bun run disk` before this):

| Where | Size | Bounded? |
|---|---|---|
| `target/crate-corpus/` | 25 GB | No: each build of the compiler added a copy of every crate's dependencies, `url`'s 190 copies, 5.8 GB; from nothing all 23 crates are 596 MB, built in 43 s |
| the rest of `target/` | 11 GB | kept apps, compile cache, test map by their own caps; `mutants/` none |
| `~/.cache/rust-js/` | 1.4 GB | yes, by `test/kept.ts`, but outside the project |
| `$TMPDIR` | 1.5 GB | No: 740 of Vitest 5's `<id>/ssr/` module caches, one a run, its `_tmpDir` never removed; 216 tests' `mkdtemp`s never removed |
| the checkout's root | 3 MB | No: 789 of rustc's `*.long-type-*.txt` |
| rustup's toolchains | 6.2 GB | No: an old pin's, 1.98.1, 2.1 GB |
| Cargo's downloads | 1.8 GB | by Cargo's own GC, by age |
| Bun's install cache | 2.6 GB | No |
| Playwright's browsers | 1.1 GB | by `playwright install`, which removes the browsers no installed Playwright uses |

## Decision

Every place development writes is one of four, each named by one owner
and reported by `bun run disk`:

1. **The main checkout's `.cache/`**, gitignored: the machine's stores,
   which every worktree shares, found through the git common dir
   (`test/bounded.ts`' `mainCheckoutOf`, without a process). It was
   `~/.cache/rust-js`: inside the project now, where it's seen. Each
   machine's old one is moved there once, by the first prune of the
   stores, every `check:fast`'s and `bun run test`'s (`test/kept.ts`'
   `adoptRetiredStores`), merged with what the main checkout keeps
   already: a directory's entries each in turn, a task's append-only
   `.jsonl` its old lines first and then the new ones, so a task started
   before the move keeps its start (ldAC's), any other file the newer; and
   `~/.cache/rust-js` removed, so no machine keeps an orphan.
2. **Each worktree's `target/`**: its build, and its stores, each with a
   cap. The crate corpus's projects get one, 2 GiB, the least recently
   measured crate first (a crate rebuilt about every third build of the
   compiler, two seconds of it); `mutants/` 4 GiB.
3. **The temp root, `$TMPDIR/rust-js-dev/`**: every test process's
   `TMPDIR` (`test/setup.ts`), its children's too, so what a test or a tool
   it runs leaves in the temp directory is held, 2 GiB, the least recently
   changed first. Not inside the checkout: a fixture made in the temp
   directory is an app outside any checkout of rust-js, which tooling tells
   apart from one inside (#1213).
4. **The tools' own, outside**: rustup's toolchains, Cargo's downloads,
   Bun's install cache and Playwright's browsers stay where their tools
   keep them, held by their tools' own commands (`bun run disk --prune`):
   - `CARGO_HOME` and `RUSTUP_HOME` are read from the environment alone;
     no `.cargo/config.toml` sets them, so a plain `cargo` or `rustup` in
     the checkout would use `~` whatever the repo's scripts set, and the
     registry and toolchains would be two, 6 GB of toolchains twice.
     `rustup toolchain uninstall` removes an exact release or dated nightly
     the repo doesn't pin, an old pin's, never a floating channel's, the
     user's own; Cargo's GC (`cargo -Zgc clean gc --max-download-size`, on
     the pinned toolchain, which its bootstrap lets take the flag) holds
     the downloads to 2 GiB.
   - Bun's install cache is every worktree's packages, cloned into each
     `node_modules`, so on their volume; `bun pm cache rm` empties it past
     4 GiB.
   - Playwright's browsers: CI installs them where Playwright keeps them,
     and every job would have to move with the path; `playwright install`
     removes those no installed Playwright uses, so they're one install.

A total budget, 40 GiB, sits above the stores inside the checkouts: their
caps, 24 GiB, a build and its fixtures, about 6 GiB, and a second
worktree's. `check:fast` prunes them as it starts, in a process of its own
while the compiler builds (1 s), and warns past 90% of the budget by the
last measure, which a detached `bun scripts/disk.ts --measure` makes for
the next run, as walking `target/` takes 3.5 s.

## Consequences

- The M1 went from the 40 GiB floor to 66 GiB free: the crate corpus's 25
  GB, the temp directory's leaked 560 MB, the root's 3 MB.
- `test/disk-owners.test.ts`: "only the stores' owners name where
  development keeps things outside a checkout", no path made in the user's
  home but `test/bounded.ts`' and `scripts/disk.ts`'; and "a test of what
  a store keeps gives it the disk's answer" covers `boundCrateProjects`
  and `boundEntries`.
- Not done here: sccache isn't installed on the M1; where it is, its
  `SCCACHE_DIR` and `SCCACHE_CACHE_SIZE` are its own, reported by `bun run
  disk` once a machine has one. rustc's `*.long-type-*.txt` are removed,
  not yet kept from being written.

## Amendment: rustc's notes aren't written at the root (2026-11-12, #1466)

The root kept filling: 657 files in one checkout, 491 `lib.long-type-*.txt`
and 178 `rustc-ice-*.txt`, removed a day later by the store above but
written again by every run.

- **A long type's name.** rustc writes a name too long for its error's
  line to a file beside what it compiles to, and names the file in the
  error. A file compiled by rust-js alone, every test's, has no such
  place: rustc's output directory is where the compiler runs. A closure's
  name holds its file's whole path, so any error naming a fixture's
  closure wrote one. `RustJs::config` turns it off for a file compiled
  alone (`write_long_types_to_disk`), and the error prints the name whole,
  as rustc's `--verbose` does. A Cargo crate keeps rustc's way: the file
  is in its target, where rustc's other outputs are. No JS changes; the
  error's text does, to what a reader can use without the file.
- **rustc's crash reports.** All 178 were one crash, rustc's MIR validator
  on a Rustlantis program's `fn0` (`validate.rs:356`), of the native
  reference build the net makes, run from the root so rustup finds the
  pinned toolchain. It runs in its fixture's directory now, inside the
  checkout too, so the report is removed with the fixture, and kept with
  it by `RUST_JS_KEEP_FIXTURES=1`. The build's stderr, which the net
  reports, has the crash already.

The store "rustc's notes at the root" stays, for what older commits'
runs and a plain `rustc` run by hand leave. `test/diagnostics.test.ts`'s
"a long type's name is printed in its error" holds the first; mutation
`long-types-written-where-it-runs`.

## Amendment: a kept compile is counted by its blocks, and the cache's tests ask it as a build does (2026-11-18)

The compile cache (test/compiles.ts) held 13.7 GiB against its 2 GiB cap,
and `bun run disk --prune --inside` left 5.1. Its writers didn't bypass
the bound; `prune` counted wrongly. `keep` named each kept compile by its
files' lengths and its output's, and `prune` trusted the name. What a
kept compile takes is its files' blocks and `entry.json`'s, which holds
every path the compile read: an entry named `-5062` took 16 KiB. The
counted 2.00 GiB took 5.12 on the disk.

`keep` writes an entry, then names it by `bytesOf` of it, as every
store of the loop's is counted (`tooling/bounded.js`), and asks `room`
with that. One way to count a store's bytes. The layout's version is 2,
so the miscounted store is dropped as any older layout is.

The cache's tests failed in a mutant's environment, mutant or not:
- the runner sets `RUST_JS_MUTANT`, which keeps none of a mutant's
  compiles, so four tests counted a compile more than they kept;
- a mutant's own target puts the verified runs in `corpus-cache`, so
  support's row of the stores' caps named the wrong one.
Each `--retire` had said them failing as the compiler is too. The cache's
tests now clear `RUST_JS_MUTANT`, as they clear `RUST_JS_DECISIONS`, and
the caps' row names the store as `test/kept.ts` places it.

test/compiles.test.ts holds a kept compile's name against its blocks, and
a store pruned to its cap at or under it on the disk. Mutation
`kept-compile-counted-by-lengths` is caught.

## Amendment: every other name in the store is a dot-name, and what's being built counts (#2107, 2026-11-18)

A key's mark of a compile rust-js can't record was a file named
`unrecordable` beside the kept compiles, `<key>-<bytes>`. The store's own
readers skip a name with no size, but the test of what a compile is kept
as read every entry new since it began in the shared store, and under
check:fast's load met another job's mark there, its size `NaN`. The mark is
`.unrecordable` now: every name in the store that isn't a kept compile is a
dot-name, as a writer's `.building-..` already was, which no reader takes
for one. That test reads its own compile alone, the entry that read its
`lib.rs`.

And what's being built, a `.building-..` of a writer at work or of one that
ended before it finished, is on the disk: `prune` counts it in the store's
size, never evicting it as a kept compile, and removes it an hour on, as
before, before it counts (`buildingBytes`). Rows: "the store's other names
are dot-names, which pruning neither counts nor removes as kept compiles"
and "a compile being built counts in the store's size till a prune an hour
on removes it". Mutations `building-uncounted` and
`building-counted-before-cleanup`.

## Amendment: a kept store's cap never takes its working set (#2344, 2026-11-23)

native-cache's cap, 10 GiB, sat below what the Mac mini's checks used,
11.5 GiB of native builds. At the disk's floor, `bun run disk --prune`
held the store to its cap, evicting builds in use, and the next
`check:fast --all` had every corpus part stop at its 560 s limit (47 s
usual): re-warming took 1,018 s of the whole machine. The budget already
kept the working set, what was used since the checkout's last check or in
the last day (#2163); a store's own cap didn't.

Now one rule holds both, `holdKept` (test/kept.ts): past a cap, the stale
entries go, the least recently used first; the working set goes only for
the disk's floor. So a store's least is what the machine's runs use,
measured at each prune, and its cap is what it keeps past that, not a
constant raised by hand as the working set grows. The working set's
definition (`workingSetSince`, `lastCheckMark`) moved from scripts/disk.ts
to test/kept.ts with it, where `pruneKept` asks it; `holdToBudget` asks
`holdKept` with the other stores' bytes beside. Row: "a kept store past
its cap keeps its working set, and only stale builds go". Mutation
`kept-cap-takes-working-set`; `disk-working-set-evicted`,
`disk-last-check-unread` and `disk-floor-spares-working-set` moved with
the code to scripts/mutations/kept.ts.

## Amendment: the floor and the caps are the disk's (#2505, 2026-11-24)

The floor was 40 GiB on every disk. On the 256 GB Macs, thor's Mac mini
(a 228 GiB volume, 42 GiB free) and lisa's M1, that's 18% of the disk, so
their stores kept nothing whenever a check's own writes took them under
it, and the tests that ask a store to keep something failed for the disk,
not the change (ioFH's push: 39.9 GB free, three files red). The native
builds' cap, 10 GiB, and sccache's default, 10 GiB, were the same on a
228 GiB disk as on a 1 TB one.

Each is now sized by the volume the checkout is on, as `machine()` sizes a
run by the cores and memory, with no list of machines (`diskProfile`,
test/bounded.ts, the one owner):

| | 228 GiB | 460 GiB (this Mac mini) | 931 GiB (1 TB) |
|---|---|---|---|
| Floor: 8%, at least 20 GiB, at most 40 | 20 | 36.8 | 40 |
| Native builds' cap: 4%, at least 8 GiB | 9.1 | 18.4 | 37.2 |
| sccache's cap: 4%, at least 8 GiB | 9.1 | 18.4 | 37.2 |
| The checkouts' budget: the native cap and 30 GiB | 39.1 | 48.4 | 67.2 |

- The floor's least, 20 GiB, is room for a check's own writes beside a
  fresh build of the compiler and macOS's swap; its most, 40 GiB, is what
  it was everywhere, which no machine ran out above.
- The budget holds the other stores' caps and a build as it did; it grows
  only by what the native builds may keep.
- sccache's cap is written to its config (`sizeSccache`, scripts/disk.ts),
  which its server reads as it starts, past its 10 idle minutes, and
  holds its cache to, the least recently used first. A config a person
  wrote stays theirs, said by `bun run disk --prune`; the one this writes
  starts with a line saying so. `check:fast`'s prune writes it, so no
  machine needs a step by hand.

A smaller floor makes it rarer, not impossible, for a disk to sit at it, so
the tests that ask whether something is kept no longer ask the real disk
either. Each function that keeps a native run, a reach or a mutant's
workspace takes the disk's free bytes from its caller, as the stores'
prunes did (#1208): `runNative`'s `free`, which its build and its run are
kept by, `nativeCratesOutput`'s, `keepRun`'s, `reached`'s and
`makeSourceWorkspace`'s. Their tests give them room, so their answer is
the code's, whatever the disk; test/disk-owners.test.ts' rule, which held
the stores' own test files to it, holds test/native-outcomes.test.ts,
test/verified-runs.test.ts and test/source-mutations.test.ts too, the
three that failed at 39.9 GB free.

Held by test/support.test.ts' "the disk's floor and its stores' caps are
sized by the volume" and "sccache's cap is the disk's, written to its
config but never over a person's own"; test/native-outcomes.test.ts' "a
native run below the disk's floor it's given is run, not kept"; mutations
`disk-floor-fixed`, `disk-floor-unbounded`, `disk-native-cap-fixed`,
`sccache-config-overwritten`, `native-run-kept-past-given-floor`,
`keep-run-real-disk`.

## Amendment: a store's cap is its working set and what it keeps past it (#2483, 2026-11-25)

The amendment of #2344 said a store's cap is what it keeps past its
working set; the code held a cap of the whole, the stale going until the
whole was under it. So a store whose working set had grown past its cap
kept nothing stale, was said past its cap by `bun run disk` whatever it
held, and its index made it due a walk every hour (#2498's `walkDue`). On
the M5 Pro (2026-11-25), every entry of each used in the last day:

| Store | In use | Cap of the whole |
|---|---|---|
| native-cache | 22.07 GiB | 18.4 GiB |
| std-reach | 438 MiB | 64 MiB |
| native-outcomes | 400 MiB | 256 MiB |
| test-maps | 1.22 GiB | 512 MiB |

Now each store of things made once and used again whole keeps its working
set and `past` bytes beside it, and its cap is the two, measured at each
walk and kept in its index (`.walked.json`'s `working`), which `bun run
disk` says beside its size. `holdKept` takes `past`, the stale's room,
where it took a cap and the bytes beside it; `holdToBudget` gives it the
budget less the rest and the working set, as it did. A store's index makes
it due an hour on only where more of it is stale than it keeps past its
working set; a working set, however large, isn't a walk's reason. The
stores with no bound of their own that hold things, the check logs, the
job times, the tasks, the sanity runs, Rustlantis' build and the temp
root, are held so too (`holdEntries`, scripts/disk.ts). The test maps,
each superseded by the next commit's, keep their count; their cap is that
many of the newest one's size (`mapsCap`, scripts/testMap.ts), not 64 MiB
a map.

A store of builds, each entry a Cargo target built in again, the crate
corpus's projects, the Crater's, the kept apps', the mutants', isn't: its
entry in use grows a copy of its dependencies with each build of the
compiler (25 GB of the crate corpus's before #1213), so its working set
says nothing of what it needs, and it's held to its cap whole until
Cargo's own units bound it (#2483's third item). The compile cache keeps
its own rule, another build's dropped whole, its 2 GB past what a day of
one build keeps (87 to 592 MiB measured).

Each store's cap by the machine, the working sets the M5 Pro's (the
floor and what the native builds keep past theirs, `diskProfile`):

| | 228 GiB (simulated) | 460 GiB (the M5 Pro) | 931 GiB (1 TB) |
|---|---|---|---|
| Floor | 20 GiB | 36.8 GiB | 40 GiB |
| native-cache: 22.07 GiB in use and 4%, at least 8 GiB | 31.2 GiB | 40.5 GiB | 59.3 GiB |
| verified-runs: 287 MiB and 512 | 799 MiB | 799 MiB | 799 MiB |
| std-reach: 438 MiB and 64 | 502 MiB | 502 MiB | 502 MiB |
| native-outcomes: 400 MiB and 256 | 656 MiB | 656 MiB | 656 MiB |
| native-sources: 30 MiB and 64 | 94 MiB | 94 MiB | 94 MiB |
| the temp root: 739 MiB and 2 GiB | 2.7 GiB | 2.7 GiB | 2.7 GiB |
| test-maps: 8 of 164 MiB | 1.28 GiB | 1.28 GiB | 1.28 GiB |
| The checkouts' budget | 39.1 GiB | 48.4 GiB | 67.2 GiB |

On a 256 GB Mac the floor is what binds: thor's Mac mini, at 42 GiB free
(#2505), has 22 GiB above its 20 GiB floor, about the M5 Pro's
native-cache in use alone, so a working set that size takes it to the
floor, where `holdKept` takes the least recently used of the working set,
as it did, and nothing more is kept; the cap never decides there. On the
M5 Pro and a 1 TB machine the working set stays whole, and a night's
builds of the net stay past it.

Held by test/support.test.ts' "a kept store's cap is its working set and
what it keeps past it, and only stale builds go", "a store is walked again
only once its index says a walk is due", "a store of things keeps its
working set and what it keeps past it, and its cap says both" and "a
checkout past the budget keeps its working set"; mutations
`kept-past-less-the-working-set`, `store-walked-for-its-working-set`,
`disk-budget-working-set-uncounted`, `things-store-held-whole`,
`things-store-cap-without-its-working-set`, `test-maps-cap-a-constant`.

## Amendment (2026-11-25, #2562): an entry a run is using is removed only as its store removes it

The stores' prune runs beside a check's jobs (#2498). On lily's M6 it removed
the crate corpus's itertools project while that run was measuring it, 3
checks of 3: "itertools' JS has no adaptors.js", and sccache's "Couldn't
determine current working directory". The prune had listed the project by
its old `used` mark, the run then began measuring it, and the prune removed
it by that listing. `holdKept`'s check before removing reads the directory's
own time, which the mark doesn't change.

A crate's project is now held while a run measures it, by its claim,
`<crate>.owner` (test/kept.ts' `takeClaim`), as a kept app's build is held
(#2446). The prune lists no held project and removes one only under its
claim, `removeCrateProject`. A store says how its entries are removed,
`Store.remove`, and the checkouts' budget removes each entry as its store
does, where before it removed every store's entries whole. One claim
protocol, test/kept.ts', is the only one, and `claimHeld` (since `isHeld`,
below) answers whether a live process holds a claim, for the kept apps' prune and the crate corpus's
alike.

## Amendment (2026-11-25): the Crater's work is held as a run uses it, by the same claim

`target/crater/` had #2562's shape: nothing held what a Crater run was
using, so the stores' prune beside a check's jobs, or its budget, could
remove the native target every crate's tests build in, or a crate's
source copy, under the run, by a listing made before the run began it.

The claim is now one mechanism, test/kept.ts', where three stores had
their own pieces of it (the kept apps' prune, the crate corpus's
`holdCrateProject` and `removeCrateProject`):

- `whileHeld(paths, use)` runs `use` with each path held by its claim,
  `<path>.owner`, waited for where another live process holds it;
- `isHeld(path)` answers whether a live process holds it, and a store's
  listing leaves out what's held and the claims themselves (`claimMaking`);
- `removeClaimed(entry)` removes a listed entry only under its claim, so
  one a run began using after the listing stays.

A Crater run holds what its crates share, `native/`, `js/`, `fetch/` and
`expand/`, for the whole run (`sharedOf`), and each crate's copies, its
source's, its JS's and that one's expansion, while it's measured (`ownOf`).
Each path the run works in is one of these, so what it holds is what it
uses. Its store's bound and the checkouts' budget remove its
entries by `removeClaimed`. A second Crater run in the same checkout waits
for the first, saying so, as two would race on a crate's copies anyway.
