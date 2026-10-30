# rust-js: north star

**Idiomatic Rust in. Readable JavaScript out.**

Compile the Rust engineers naturally write into correct JavaScript people
would write by hand. The goal is Rust across the stack: native on the
server, rust-js on the frontend, shared types, and Scala.js-level
correctness, libraries and interop. The first production app is a milestone.

Be ambitious: change any representation, analysis or binding needed to
serve this goal. Size alone is no reason to settle. Make changes safe with
a failing test, native comparisons, mutations and an ADR explaining why.

## Correctness first

**The JavaScript must do what the Rust does. No rule outranks this.**

- Never trade correctness for readability. Emit the correct form or refuse
  clearly at compile time; never approximate or emit partial output.
- A wrong answer is the worst bug. Fix it before other work, or file it
  immediately and report it.
- Prove behavior against native Rust using the corpus, rustc's tests,
  generated programs and mutations. Inspect generated JavaScript too.
- Preserve every observable detail: evaluation order, side effects,
  copying, overflow, errors, panic messages, drop order and iterator behavior.
- rustc owns types, traits, ownership, borrowing, validity and diagnostics.
  Never bypass its checks.

For Rust/JS differences, apply [ADR 0262](docs/decisions/0262-when-rust-and-js-disagree.md):

| Observable difference | Decision |
|---|---|
| None | Use JavaScript's behavior; explain why. |
| Rare and harmless | Only an explicitly decided, documented and tested exception, listed in ADR 0262's Consequences and the guide; never decide alone. |
| Results change | Preserve Rust's behavior, however the output reads. |
| Rust's behavior cannot be preserved | Compile error. |

State which case each decision follows.

## Idiomatic Rust in

**Write the Rust an experienced Rust engineer would naturally write.**
Use familiar types, APIs and patterns taught by the Rust book, std's docs
and clippy. The compiler adapts to idiomatic Rust, not the other way around.

- Use std's types and idioms: `Vec`, `HashMap`, `Option`, `?`, iterators,
  `impl Trait`, ownership and borrowing. No rust-js substitutes or dialect.
- rust-js-specific APIs belong only at the JS boundary: bindings, JS value
  types and `jsx!`.
- Fix the compiler when idiomatic Rust fails or produces unreadable JS.
  Never reshape an app, narrow types, add clones or replace an iterator
  with an index loop to accommodate lowering.
- Examples, tests and ports must not teach compiler workarounds. Format
  with rustfmt and satisfy rustc's and clippy's defaults without `#[allow]`
  added for rust-js's sake.

## One way, one authority

**Within choices rust-js owns, this takes priority after correctness.**
Provide one API, attribute, macro, binding form, option or config key per
job; one output form per construct; one semantic owner and catalog per rule;
one way for hosts to build.
This does not restrict Rust's established alternatives: loops and iterator
chains both belong. Ease of lowering must not dictate the source.

When two ways or competing authorities appear:

1. Stop; do not build on either or add a third.
2. Choose the one that covers every case, reads best and keeps Rust's checks.
3. Migrate consumers and remove the other. Reject its use with a compiler
   error or test failure naming the supported way.
4. Prevent recurrence with tests and record the choice in an ADR; then resume.

Never add synonyms, aliases, fallbacks or "also accepts" for convenience.
Extend the existing form. If removing duplication seems disproportionate,
stop and ask the user; do not silently leave both.

**Define each semantic rule once; consume its answers everywhere.**

- Find its owner in [the architecture](docs/architecture.md). Extend that
  owner, or give a new question an API in the responsible module. Owners
  may consult other owners; one authority does not mean one giant module.
- Share knowledge, not just code: different-looking implementations can
  duplicate a rule; similar code for different rules need not be combined.
- Carry decisions through analysis, lowering, linking and printing as APIs
  or explicit facts. Never reconstruct them from names or JS shapes outside
  the representation owner's API. For example, copying decisions belong
  to `copies.rs`.
- Hosts consume compiler decisions through the manifest; build and packaging
  use shared catalogs. If sharing across a boundary is impossible, document
  the authority and reason, and test the necessary mirror by running both.
- Caches, indexes and serialized facts must derive from the owner, with
  explicit source, input keys and invalidation/version checks. No second
  editable catalog or fallback policy.
- Enforce contracts with types and visibility, ownership with
  [architecture tests](test/architecture.test.ts), and behavior with tests.
  Update the architecture and an ADR when ownership changes.

Red flags: two places must change for one rule, repeated conditions or
catalogs, "mirrors" comments, or downstream reconstruction. Apply the
stop-and-resolve procedure above.

## Simple boundaries, unsurprising behavior

- Each module owns one responsibility behind an explicit contract. It
  should be understandable without knowing unrelated phases' internals.
  A pass working around another phase's representation needs a shared owner
  and an architecture test holding the boundary.
- Make each API guessable from one example and consistent everywhere.
  Follow Rust conventions in Rust and JS/React/TypeScript conventions in
  output and interop.
- Extend a general rule instead of adding exceptions. If none can cover
  the case, ask the user; document accepted exceptions where encountered
  and reject violations with an error explaining what to write.
- Treat generated names, control flow, modules, JSX, formatting and source
  maps as product features. Use objects, arrays, functions, promises and ES
  modules; emit helpers only when behavior requires them.
- Browser APIs, React, npm and Vite are core. Preserve public representations
  and interfaces, and keep native and browser compilers consistent.

## Ports expose compiler gaps

Port TypeScript/JavaScript projects, react.dev first, to discover what
rust-js lacks. Preserve behavior and make output as close to the original
as idiomatic Rust allows.

**Stop the port and fix rust-js** when encountering:

- Unsupported valid Rust or a missing compiler feature.
- Missing js/webapi/react bindings, or types narrower or looser than the
  original (including `any` or missing `| undefined`).
- Output differences such as renames, boxes, helpers, casts,
  copies, lost names or statements replacing expressions.
- Rust bent to fit the compiler: narrower types, leaks or restructured code.

Start with a failing test, fix the compiler, add mutations and an ADR, then
resume the natural port. Never hide gaps with local stand-in bindings,
`#[link_name = "this"]` casts, source workarounds or accepted noisy output.
"Cosmetic" and "correct anyway" are not exemptions: if a fix seems
disproportionate, ask the user and record the decision.

Before committing a port, compare **every generated file line by line**
with its original. Fix each difference in rust-js unless the user has
explicitly accepted that difference.

## Keep the guide true

[The guide](docs/guide/README.md) is the authority on what to write today:
Rust constructs, JS output, bindings, exports and refusals. ADRs explain
why; later decisions may supersede earlier ones.

- Read the guide before writing Rust for rust-js, especially ports. Fix
  missing or incorrect guidance; never work around it.
- Update relevant guide pages in the same commit as changed decisions,
  source conventions or output behavior.
- Never handwrite the guide's generated JS. `bun run docs` compiles it;
  `test/docs.test.ts` checks that it matches the compiler.

## Making and checking changes

Follow [CONTRIBUTING.md](CONTRIBUTING.md) for implementation, proof and
documentation, and [the architecture](docs/architecture.md) for placement.

Commit on `main` and push directly: **no branches or pull requests**. If
a push fails, keep committing locally and push later
([DEVELOPMENT.md](DEVELOPMENT.md#working-on-main)).

**Every local command has a timeout that fits it**, about twice what it
usually takes, enforced by the tool or `timeout`, so a hang ends instead
of being waited on. Run what makes the loop fastest, with all of the
machine's cores: a whole suite in parallel parts here is fine where it's
quicker than CI's round trip. A command past its timeout is a hang or a
slow spot: find out which and fix it, or list it in
[DEVELOPMENT.md](DEVELOPMENT.md#slow-spots-still-to-fix); never just
retry it with more time. This overrides other instructions here and in
CONTRIBUTING.md.

| To | On the Mac | On CI |
|---|---|---|
| Test | `bun test test/<file>.test.ts -t <name>` | `bun run test` |
| Bless snapshots | `BLESS=1 bun test test/<file>.test.ts -t <name>` | `bun run ci:bless` applies its patch |
| Mutations ([ADR 0093](docs/decisions/0093-mutations.md)) | `bun scripts/mutations.ts <name>` | changed ones; all before a release |
| rustc tests ([ADR 0089](docs/decisions/0089-rustc-tests.md)) | `bun run test:rustc drop/ closures/` | all; bless lists with `bun run ci:bless` |
| Generated programs ([ADR 0092](docs/decisions/0092-generated-programs.md)) | | `gh workflow run "rustc tests" -f fuzz_start=1000 -f fuzz_seeds=600` |
| Playground compiler | | check's `wasm` job |

Before pushing, run `bun run typecheck`, `bun run fmt:check`,
`cargo clippy --locked -- -D warnings`, and the change's tests and mutations.
A push to `main` starts its own check, which runs to its end: read it
with `bun run ci:status`, and don't start another after pushing, which
only waits behind it. `bun run ci:check` is for another branch, or a
run with other settings. Other workflows are
[started by hand](DEVELOPMENT.md#workflows).

**Keep the loop fast.** A slow step is paid on every change, by every
agent. Check with `bun run check:fast`, whose timings warn when a step is
slower than its usual, and read [DEVELOPMENT.md](DEVELOPMENT.md#keeping-the-loop-fast)
before long work. When something is slower than it needs to be, fix it,
or add it to that page's list of slow spots with its cost and what would
fix it, so the next agent starts from what you learned.

Keep [TESTS.md](TESTS.md) accurate in the same commit when suites, test files,
workflow jobs, counts or CI times change noticeably. Update its table and
date using its recount instructions. Local setup is documented in
[DEVELOPMENT.md](DEVELOPMENT.md#the-mac-with-the-scan-off).

Install [sccache](DEVELOPMENT.md#a-compile-cache-shared-by-the-worktrees-sccache)
on each machine, so worktrees reuse each other's compiled dependencies. It
never sits in rust-js's own compile to JS. If a build fails in one worktree
and not another, or locally and not on CI, rebuild with `RUSTC_WRAPPER=`
first; if the cache was the cause, clear it and file an issue at once.

## Keep the disk free

Check `df -h ~` before long work. Clean up periodically and at task end:
remove your merged worktrees' `target/` and finished agents' worktrees with
`git worktree remove` (the branch stays). Never remove another session's
worktree or uncommitted/unmerged work; commit outstanding work first.
