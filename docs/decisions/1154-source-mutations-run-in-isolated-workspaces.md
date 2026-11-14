# 1154. Source-owner mutations execute isolated working-tree inputs

Status: Accepted (2026-11-15), independent core validated. Amends [0093](0093-mutations.md) and [1146](1146-concurrent-checkouts-share-the-machine.md). Issue #1848.

Case: N ([0262](0262-when-rust-and-js-disagree.md)). This changes development tooling, not Rust/JavaScript behavior.

## Context

Compiler mutations rebuild an isolated Rust crate and prove their tests use the supplied compiler. That does not exercise a changed TypeScript reader or source generator: tests run from the checkout and would import its original modules. Pure parser tests correctly pass with a broken Rust compiler, so that existing negative control is insufficient for source owners. A listed mutant whose file never executes proves nothing.

## Decision

The existing scripts/mutations.ts remains the only runner, selection catalog, lease, worker pools, test target isolation, logging and verdict authority. scripts/source-mutations.ts owns only independent source workspaces and actual-load proof. Each nonempty mutation list names one real existing repository source file through its file field; no src-prefix reconstruction determines ownership.

For non-crate ECMAScript/TypeScript module owners, take one unchanged template of the tracked working tree, excluding generated target, node_modules and Git stores. Clone independently writable files for each control/mutant, using filesystem copy-on-write when available and ordinary copies otherwise. Never share writable source inodes. Installed third-party dependencies are read-only inputs; every workspace package alias points to that workspace's own package, derived from the workspace manifests. A whole shared node_modules would resolve original @rust-js/typescript and is not allowed.

Append a real execution marker to the owner, preserving existing line numbers, directives and shebangs. A missing marker makes the run inconclusive. Each unchanged control must execute its owner and pass actual tests. Each mutant names the single real test that must catch it. A timeout, unloaded owner, failed prerequisite, failed control or runner failure is not caught. The existing judge remains the verdict owner. CommonJS owners are explicitly unsupported by this module instrumentation path until their own correct load form is added.

Source tests reuse a read-only compiler binary. Pure module mutants occupy the existing test-slot budget, without a compiler rebuild. Generator-chain mutants declare one prepare command and occupy an existing build worker: the generator writes its canonical Rust output in the workspace, then the real named binding test rebuilds crate metadata in that test run's own target. The workspace printer is selected explicitly through RUST_JS_TYPESCRIPT. There is no second source-rewrite or compiler-build mechanism.

The normal checkout mutation lock protects target/mutants. Leases inherited by children are unchanged. Workspaces stay alive through their tests, are removed on completion/error, and the locked run cleans orphaned source workspaces at startup and process exit before releasing its lock. Their disk peak is measured and checked against the existing development disk floor; target/mutants remains counted by disk.ts.

## Proof and costs

The first three real TypeScript owner mutants were caught through the normal runner, with executed unchanged controls and real load markers: omit construct decoding, print it as a call signature and accept an unknown Member kind. They ran together in about 22 s while another checkout was also validating. Each active workspace measured90,992,640 bytes; per-mutant preparation4.8–5.6s and tests 9.3–12s. These are initial contention measurements before the unchanged-template refinement, not an isolated performance comparison.

In the combined private Intl/model plus harness review, after the unchanged-template refinement, the normal runner caught all 15 registered owner mutants in about 62 s: four Intl generator chains, three constructor inventories, three TypeScript model rules and five source-runner guards. The template measured 91,021,312 bytes and 1.6 s; each workspace peaked at91,025,408 bytes. Generator preparation 4.9–9.5s and tests 22–28s were measured in the concurrent 15-mutant batch, not isolated per-mutant benchmarks. Real subprocess tests cover unloaded owners, already-failing controls and setup-error cleanup. The unchanged baselines pass 11 tests with 42,367 assertions. Latest-base adoption to 0ab884b passes the focused checks. The independent core passes check:fast --all on 4652f87731e98db8f21b493f74c5a2917d128fc6 in 7 min 57 s: fourteen corpus parts, eight browser files, 296 other test files, the five self-mutants, Clippy, formatting, cargo tests, Vite and matrix samples. Owner registrations for the Intl/model changes are packaged separately so the harness core can land independently. DEVELOPMENT.md and docs/dev-loop.md record these measured costs; no pre-existing equivalent harness time or isolated speed-up is claimed.

## Alternatives

README-only kill recipes are not durable CI proof. Registering JS/TS files without changing test imports would falsely judge mutants of original sources. Editing checkout sources would race other validation. A separate mutation runner would duplicate selection, scheduling and verdict policy. This design reuses those existing owners and adds the missing source-execution boundary.

## Known limits (at landing, 2026-11-15)

- **A third-party package's own imports of a workspace package** resolve
  through the checkout's `node_modules`, so they read the original
  source, not the mutant's copy. The workspace links its own packages for
  the checkout's code, not inside the third-party packages it links
  read-only. No registered mutation reaches a workspace package that way.
  A mutant whose tests reach its owner only through such a package isn't
  proved loaded by that route. The load marker still tells whether the
  copy ran.
- **A generator's preparation** is the generator alone. The js crate it
  changes is rebuilt in the mutant's test run, under that test's timeout.
  Alone in a quiet workspace it takes 18 ms, after a 1.9 s copy of the
  workspace; the template on main takes 0.01 s warm, cold and fresh. It
  first landed with a 1 s timeout, from that. Three of ten Intl mutants run
  at once then timed out at 1 s, which the quiet measurement doesn't
  explain: #1861 is finding why, a slow spot listed in docs/dev-loop.md.
  Until then the timeout is 20 s as the load makes it (`underLoad`), twice
  the 9.5 s the measured step took at most, the copy with it, in the
  15-mutant batch beside another checkout's check. Past it is a hang,
  reported as `INCONCLUSIVE: source preparation timed out after N s`,
  apart from a preparation that failed.
