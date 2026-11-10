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
