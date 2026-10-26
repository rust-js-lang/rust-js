# Tests at a glance

As of 2026-10-24, `main` at `2466f604`; times are CI's, check run 40068253808.

| Suite | Size | What it proves | Time |
|---|---|---|---|
| `bun run test` | 2,701 tests in 144 files | Everything below except mutations, rustc and fuzz | 13 min on 2 machines (~16 min on one) |
| ↳ corpus | 889 Rust programs (`test/corpus/*.rs`) | Each one's JS prints what native Rust prints, and matches its snapshot | about half the suite |
| ↳ the other 143 files | ~1,800 tests | Lowering rules, diagnostics, bindings (js, webapi, React, Node), hosts (Vite, Next, cargo), React's ported modules against their snapshots, docs, architecture | the rest |
| `cargo test` | 48 unit tests | Tidy's passes and decisions | <1 min |
| Mutations | ~5,100 (`scripts/mutations/`) | Each rule has a test that fails without it | a check runs only the changed ones: ~10 min on 8 machines |
| rustc's suite | 3,762 run-pass tests, 1,493 of them out of scope | rustc's own programs run the same as JS | a check runs a sample in 1 min; all of them take ~2 h 45 min |
| Generated programs | 600 random programs a night | Programs no one wrote | in the nightly |
| Tidy's stages (`test/tidy-stages.test.ts`) | 6 corpus cases in the suite; every case a night, each at 16 stage ends | Each of tidy's stages leaves JS that runs as native Rust does (ADR 0659) | ~20 s in the suite; in the nightly, on 6 machines |
| Iterator forms (`test/iterator-forms.test.ts`) | 12 corpus cases in the suite; every case a night, built with std's iterator structs alone (`--std-iterators`) | That build runs as native Rust does, but the cases `differs` lists, which only shrinks (ADR 0908) | ~20 s in the suite; in the nightly, on 2 machines |

- **A check** (each push to `main`, which runs to its end; `bun run ci:check` of another branch) runs everything except the full rustc suite and fuzz, in parallel: about **15 min**.
- **The nightly** runs every mutation, the 600 programs, every corpus case after each of tidy's stages, and every case with std's iterator structs alone: **~2–3.5 h**.
- **On the Mac**, each command is capped at 5 minutes: run one file, or `-t <name>` (AGENTS.md).

To recount: the test totals are the `Ran N tests` lines in a check's `test (k/2)` logs. The corpus is `ls test/corpus/*.rs | wc -l`, the mutations are `grep -rc 'name:' scripts/mutations`, and the rustc counts are the line counts of `test/rustc-*.txt`.
