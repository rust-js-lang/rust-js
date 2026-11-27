# 1210. A change's dependants are what the map recorded reaching it

Date: 2026-11-27.

Status: accepted (#2482: the loads, in shadow; #2608: the edges, to
come). At each source's switch it supersedes part of
[1101](1101-the-inner-check-runs-what-a-change-reaches.md):
- the edges replace its name search for a line outside any function's
  counted code: `byOneName`, `seenFrom`, the short-name and N-of-its-crate
  valves (the amendments of #2288, #2523 and #2608);
- the loads replace the folder row of its 2026-11-16 table, "a package's
  or crate's folder runs everything".

Case ([0262](0262-when-rust-and-js-disagree.md)): none. This is about
which tests a change runs, not about what the JS means.

## Context

Two of the selector's rules (scripts/select.ts) guess at what reaches a
change, and each guess, when it fails, runs everything.

- **A file of a folder tests build an app from** (`tooling/`, `browser/`,
  `wasm/`, `examples/`, `serde/`, …) ran every test. No import says which
  test reaches it: a test that builds an app through `@rust-js/build`
  loads `tooling/build.js` in a child process. Of main's checks of
  2026-11-21 to 11-24 (#2288's count), 14 runs selected everything for this
  reason, 276 runner-minutes, 10 of them the change's own.
- **A line of the compiler outside any function's counted code** is
  searched to the functions that name its item, by the item's name as
  text. Each dead end of that search selects everything: a name too short
  (`Cx`, `Of`), or an item not found. 14 of the 16 compiler commits
  replayed after the map at ac9716c758 did (#2608).

The test map already records what reaches each test, for the compiler's
functions (ADR 1101). This decision records the other two sources with it,
and has one module answer from them.

## Decision

**One map, its lists keyed alike.** The map (`mapVersion` 5) keeps one
table of the checkout's files, `paths`, sorted. Every list of files the
map keeps names files by an index of it.

**Source 1, the loads (#2482, this change).** `loads.of[testFile]` lists
the files of `paths` that a test file loaded, in its own `bun test`
process and in every child it started, Bun's and Node's
(scripts/testMapLoads.ts):
- the test process: a `--preload` whose `afterAll` writes `require.cache`
  (`bun test` fires no `exit`);
- each Bun child: `BUN_OPTIONS=--preload`. It takes itself out of
  `process.execArgv`, which Next.js's dev server gives its workers: with a
  preload there, `next dev`'s page never rendered. Its list is written
  each second it grew and at `exit`, so a server the test kills still
  leaves one;
- each Node child: `NODE_OPTIONS=--import`, with `module.registerHooks`,
  appending each module as it loads.

Each process writes to the folder `RUST_JS_TEST_MAP_LOADS` names, the
unit's, and each child inherits it. That's how a child's loads are its
test's.

- **A Vite config** is bundled to `node_modules/.vite-temp/<name>.timestamp-….mjs`,
  imported, and removed. It's recorded by what its bundle was made of, the
  files its inline source map names: `browser/vitest.config.ts` for the
  browser tests. The loads are what selects a Vite config: every test
  that runs it, wherever its config is found. No rule of the config's own
  remains. A test that names the config as text is its reader too, as it
  is of any file (1101's amendment of 2026-11-09).
- **A list is `null`, unknown,** when its test failed, or when one of its
  processes started a JS runtime whose environment drops the folder (it
  then writes `lost-<pid>.txt`). Such a test runs on every change a load
  selects.
- **A test recorded in parts** gets its parts' lists together. An
  incremental recording carries each test it doesn't record from the map
  it carries from, as it carries their functions.
- **The map's own compiler wrapper**, one Bun process for each compile,
  records nothing.

**Source 2, the edges (#2608, worker-02's).** These are rustc's resolved
references, recorded by a driver in the counting build. `defs` holds each
of the compiler's definitions: `{ file, lines: [start, end], fn? }`,
`file` an index of `paths`, `fn` its function's index where it's one.
`edges` holds an `Int32Array` of pairs of `defs` indices (the namer, then
the named), or `null` where the pass didn't run: no edges arm.

Both fields are reserved in the map's type, and this section is worker-02's
to fill as the pass lands. A line outside any function's counted code
selects the functions that reach its item through the edges, walked
backwards, and through `units` the tests that ran them.

**One lookup, scripts/dependants.ts.** `dependantsOf(changes, { files,
map, reach })` gives back the test files a change reaches and why, or
everything and why. Three arms are unioned:
- `readers`: each test that imports the file, transitively, or reads it
  as text (#2480, 1101's amendment of 2026-11-09). Moved here from
  select.ts. Of a folder tests build an app from, it's what finds an
  example's sources: the tests copy those to their target, and a bundler
  reads the copy, so no process loads the original;
- `loads`: the tests that loaded the file. Of a file that's no module, a
  list or a doc (`tooling/compat/covered.txt`), the tests that loaded
  anything under its nearest folder that has loads;
- `edges`: reserved for #2608.

Unsure means everything, said why:
- no map, or a map without loads;
- a file Cargo builds as a test's dependency, which no process loads: a
  `.rs` source, a `Cargo.toml`, a `Cargo.lock`, a `build.rs`, `.cargo/`.
  When the edges' pass runs on those crates, this `all` goes;
- `.cargo/config.toml`, which every Cargo build reads, is in `everything`;
- a module no test loads, imports or reads, a new one too, until a map has
  seen it.

**In shadow, then switched.** `select.ts`'s `loadsArm` is `"shadow"`:
- the selection that runs keeps the folder rule;
- `Selection.shadows.loads` is the same selection with the loads asked,
  computed beside it. `shadows` is typed `{ items, loads, edges }` from
  the start, and the item layer's moved into it;
- each push's line says each shadow and its misses, the tests that failed
  in the check that it left out (`[loads: some (…), missed 0]`). CI's
  `selection` artifact keeps them, and `bun scripts/selector-audit.ts <dir>
  --shadow=loads` asks of the night's failures whether each check's loads
  selection ran them.

`loadsArm` goes to `"on"` after about 30 pushes and 3 nights with no loads
miss. The edges follow the same path.

## Why a skip is sound

A test runs the JS it loads. A file it didn't load, changed, can't change
what it runs, unless it reads that file as text (the readers' arm), or a
process it started loads it unrecorded (`null`: it runs). For a test T to
load a file at the working tree that it didn't load at the map's commit
`M`, T itself or a file T loaded at `M` must have changed: an import, or
a path it's given. That change selects T. So staleness is per test, as it
is for the compiler's functions.

What this can't see: a bundler's read of a file it doesn't execute, other
than a Vite config's bundle (an example's sources, found by their readers);
and a child started by something other than a JS runtime with an
environment of its own (a shell given a fresh `env`). The shadow's misses
and the nightly's audit are the backstop. Tu's direction for #2482 is a
heuristic with those behind it.

## Measured

- **The recorder's cost** (M3 Max, 14 cores, Bun 1.4.5, Node 24.17):
  - #2482's step 1, 37 files of the push suite, 3 reps under the map's
    run: +1.5% (paired reps +0.3% to +3.3%);
  - the recorder as built, on 18 app-building files back to back, plain
    then recorded: 814 s and 819 s of file time, +0.6%. Every file
    passed, `next.test.ts`'s `next dev` too, and none lost a child.
  - Its lists are a few KB a file.
- **The browser test's list** has `browser/playwright.config.ts`,
  `browser/rust-tests.spec.ts` and `browser/vitest.config.ts`, the last
  mapped from Vite's bundle.
- **`tooling/build.js` is loaded** by vite, next, deep-chain-next, docs,
  pilot, settings, cargo, playground, host-boundaries and
  rust-js-test-runners, most of them in children.

## Alternatives

- **Bun's `--coverage` lcov**: +0.5%, but only the test's own process.
  `tooling/build.js` reached 5 app-building tests only through children
  (#2482, step 1).
- **`NODE_V8_COVERAGE`**: Node's children alone, +4.3% overall (+30% for
  some files), and 300 MB of JSON for 37 files.
- **One record a test, `{ rust, loads, edges }`**: the edges are the
  commit's, not a test's, and folding `units` in would rewrite `pick`, the
  carry and `remapBits` for nothing.
- **The folder rule with readers alone**: misses every test that builds an
  app in a child.

## Consequences

- `mapVersion` 5: every older map is dropped, each machine records or
  fetches a new one. Whichever of the loads and the edges lands first
  takes 5, the other 6.
- The map's run spends about 1% more for the recorders, and the map grows
  by its `paths` and each test's indices, some hundreds of KB.
- Mutations hold each rule:
  - scripts/mutations/dependants.ts: `dependants-folder-rule-back`,
    `dependants-loads-dropped`, `dependants-unknown-loads-skipped`,
    `dependants-cargo-builds-loaded`, `dependants-folder-loads-dropped`;
  - scripts/mutations/select.ts: `loads-shadow-not-logged`, and
    `select-package-folders-read-only`, now the switch;
  - the recorder's seven: `loads-node-children-unrecorded`
    (test-map-loads.ts), `loads-preload-forwarded` (test-map-loads-bun.ts),
    `loads-vite-config-unmapped` and `loads-lost-child-unsaid`
    (test-map-loads-shared.ts), `loads-failed-test-known`,
    `loads-parts-not-joined` and `loads-not-carried` (test-map.ts).
