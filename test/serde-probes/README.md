# serde / serde_json probes for rust-js#1532

One-statement programs compared with native serde_json, and the benchmark of rust-js#1542.

- `probes-1.ts` … `probes-5.ts`: run each with `bun` in an empty directory; they write `p/<name>.rs`. Deterministic.
  (`probes-4.ts` and `probes-5.ts` read the shared header out of `probes-3.ts`.) `drop_0N.rs` go in `p/` as they are.
- `run-one.sh <name>`: compiles one natively (the pinned serde) and with rust-js, runs both, writes `out/`.
  `ls p | sed 's/\.rs$//' | xargs -P 6 -n 1 ./run-one.sh`
- `outcomes-at-f3f4bfd25.txt`: what each gave at that commit, Bun 1.4.4: `same`, `refused`, `compiler-panic`,
  `different` (a wrong answer or a runtime throw) or `native-invalid` (my mistake; rustc rejects it).
  `std_29` is `different` by design: `usize` is 32 bits here.
- `bench.rs`, `bench-harness.js`: the document of #1542. Compile `bench.rs` with rust-js to `bench.js` beside the
  harness; `bun bench-harness.js <items> emit`, then `<items> <write|read|readValue|writeValue|parse|stringify> <reps>`
  under `/usr/bin/time -l` for peak memory. 4,000 items are about 1 MB.
- `size-programs.ts`: the programs of the bundle-size table (0, 1, 10 and 50 types; write only, read only, both).

Not covered here, and covered by the independent review's issues: stateful callbacks, drops, error recovery in
hand-written impls.

## In the repository (#1532, S0)

These files are the review's as written above, kept unchanged, with `outcomes-at-f3f4bfd25.txt` its record of that commit.
`bun scripts/serde-probes.ts` (after `cargo build`) is the one bounded command that reproduces the baseline:
it makes `p/` with the generators in `target/serde-probes/`, builds each program natively with the pinned serde
(`serde/Cargo.lock`) and with rust-js, runs both on Bun and on Node, and compares each with its record, as
`run-one.sh` does one. Each program is built three ways, each with its record: as a program (`outcomes.txt`); as a
library whose `entry` runs its `main`, as the corpus builds one (`outcomes-library.txt`); and split, its items moved
into a library crate of their own by the metamorphic net's `library` rewrite, the library compiled first and the
program against its manifest and metadata, a producer and its consumer each compiled on its own, natively too
(`outcomes-split.txt`). It prints the source of each program whose outcome changed, and fails where one
that gave native Rust's answer no longer does. Name programs to run only those; `--record` writes this run's Bun
outcomes to the records, of every program. It runs as many programs at once as `atOnce` says of tests.

`outcomes.txt` at #1540's fix, Bun 1.4.4 and Node 24, the same on both, of the generated 358: 259 same, 81 refused,
5 compiler panics, 10 different, 3 native-invalid (`more_28`, a stateful skip predicate, gives native's answer since
#1543; `drop_03`, a read that fails partway, is refused since 069058dab; `drop_02`, a guard beside a hand-written
`Deserialize`, is built since #1554; an internally tagged newtype variant of what isn't a struct, `ienum_*`, `chk_*`
and `more_13`–`18`, is written as serde's `TaggedSerializer` writes it since #1540). Of the review's 28 `different`,
13 now give native Rust's answer and `pos_19` and `drop_03` are refused.

Each issue the independent review filed is a probe of its own, `issue_<n>.rs`, its repro as the issue gives it, its
first line naming the issue; the runner copies them into `p/` beside `drop_0N.rs`. Their outcomes, of the baseline:

| Issue | Probe | Outcome |
|---|---|---|
| #1536 (closed; its flatten comment) | `issue_1536.rs` | same |
| #1543 (closed) a stateful `skip_serializing_if` | `issue_1543.rs` | same |
| #1544 `expecting` run on a successful read | `issue_1544.rs` | different |
| #1545 (closed) a nested serialization `Err` uncaught | `issue_1545.rs` | same |
| #1546 `from_value` calls `visit_str` | `issue_1546.rs` | different |
| #1547 a custom `serialize_u128` past `to_value`'s range | `issue_1547.rs` | different |
| #1548 (closed) an internally tagged payload's hand-written `Serialize` | `issue_1548.rs` | refused |
| #1549 a custom option `Visitor` under `from_value` | `issue_1549.rs` | different |
| #1550 (closed) a codec looked up without its generic arguments | `issue_1550.rs` | same |
| #1552 a read that fails partway drops nothing | `issue_1552.rs` | refused |
| #1554 (closed) `impl Drop` beside a hand-written `Deserialize` | `issue_1554.rs` | refused, as it reads a `Drop` type, #1552 |

Each way's outcomes at #1540's fix, the same on Bun and Node: as a program, 263 same, 84 refused, 5 compiler panics,
14 different, 3 native-invalid; as a library, 263 same, 84 refused, 5 compiler panics, 14 different, 3
native-invalid; split, 248 same, 103 refused, 5 compiler panics, 10 different, 3 native-invalid, none `unsplit`. No
program gives native's answer as a program and another answer either other way. Since #1554 a serde backend's
associated type drops nothing, so the 62 (63 split) a library refused for the header's generic `deserialize_with`
helper `down` are built; and since #1540 an internally tagged newtype variant's payload is written by its own writer,
so the `ienum_*` a library refused (#1559) and `chk_04`, which panicked the compiler as one (#1558), give native's
answer. What's refused only split: a library's hand-written `Serialize` or `Deserialize`, #1560.

`std_29`'s answer is `usize`'s width, 32 bits in JS, so its native build is for wasm32-wasip1, as the corpus's
`//@ native: wasm32` cases are (`serde/build.sh --rlib-wasm32`, the runner's `wasm32`): against it, it gives native's
answer each way, where against a 64-bit oracle it was `different` by design.

## The benchmark (#1542, #1532's S0)

`bun scripts/serde-bench.ts` reproduces #1542's tables, the settings S2 and S5 are judged by fixed in it before any
result is: `bench.rs`'s document at 4,000, 40,000 and 400,000 items (about 1, 10 and 96 MB), written and read as a
`Doc` and as a `Value` by rust-js's JS on Bun and Node (`bench-harness.js`), by native serde_json built optimized,
serde too (`bench-native.rs`, `serde/build.sh --rlib-release`), and by each engine's `JSON.parse` and
`JSON.stringify`; then `bench-shapes.rs`'s shapes at those sizes, deep, wide, long strings and maps, typed and as a
`Value` (`bench-shapes-harness.js`, `bench-shapes-native.rs`); then each of `size-programs.ts`' programs bundled,
minified and gzipped, with its compile time. Each row cold, one operation in a fresh process, and warm, the best of
five in one, each three times, every sample kept, peak memory by `/usr/bin/time -l` beside each runtime's with no
work. `--quick` runs 1 and 10 MB once each. Raw samples and versions go to `target/serde-bench/results.json`, the
tables to `target/serde-bench/tables.md`. Its baseline is measured on a quiet machine, not one under load.

