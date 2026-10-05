# Contributing to rust-js

**Rust in. Readable JavaScript out.** Keep compiler code simple, preserve
Rust's checks, and treat generated JavaScript as part of the product.
These guidelines apply to contributors, maintainers, and AI coding agents.
The project's north star and principles are in [AGENTS.md](AGENTS.md); how
to set up and run its checks is there too.

## Correctness first

**A wrong answer is worse than a refusal.** Generated JavaScript either
behaves as the Rust does, or the compiler refuses the program with a clear
error that names what isn't supported. Never emit JavaScript that runs but
behaves differently, except for the deliberate differences listed in
[the semantics](docs/semantics.md), each documented and tested.

- **Never bypass rustc's checks** to make a feature compile, and never
  publish partial output when compilation fails.
- **A refusal that turns into a crash or a different answer is a
  regression**, even where the feature was unsupported before. rustc's own
  tests record how each listed test fails, and fail the run when one gets
  worse ([ADR 0089](docs/decisions/0089-rustc-tests.md)).
- **Known-failure lists only shrink.** A change may take tests off
  `test/rustc-known-failures.txt`; adding one needs a reason in the change.

## Architecture principle

**Understand each piece through its responsibility and contracts.**

Organize rust-js so someone can understand and change one piece by reading
its code and the contracts it uses, without learning unrelated implementation
details. Each piece should know only the concepts necessary to do its job.

A boundary does not mean having no dependencies. It means depending on clear
contracts rather than another piece's internals. For example, a printer needs
to understand the JavaScript tree and source locations. Rust trait resolution
and choosing runtime dependencies belong to earlier work.

When contributing code:

- **Give each piece one coherent responsibility.** Make clear what it
  receives, what it produces or changes, and what it guarantees. Splitting
  files alone does not create boundaries.
- **Give each decision and state one owner.** Other pieces ask that owner
  through a clear API. Do not duplicate its rules or reach into its internal
  state.
- **Pass only what the work needs.** Prefer explicit inputs and narrow
  contexts over a large compiler context that exposes unrelated capabilities.
  Query code receives facts and query capabilities; emission code receives
  emission capabilities.
- **Keep dependency direction explicit.** Follow the layers and permitted
  dependencies in [the architecture](docs/architecture.md). Put coordination
  in the piece responsible for orchestration, and name it there as an
  exception. Avoid cycles between responsibilities.
- **Carry decisions forward as explicit data.** A later phase consumes an
  earlier phase's answer rather than reconstructing it from names, generated
  text, or implementation details.
- **Keep boundaries small and meaningful.** Introduce an abstraction when it
  hides a real responsibility or protects an invariant. Avoid wrappers,
  forwarding APIs, and frameworks that merely move code around. Where
  forwarding is worth having, such as `shortcuts.rs`, keep it in the place
  the architecture names, and let it only shorten a call, never decide.
- **Protect semantics across boundaries.** Make evaluation order, side
  effects, representations, and failure behavior explicit in contracts.
  Architectural improvements must preserve established behavior.
- **Make important boundaries enforceable.** Use Rust's types and visibility
  where practical, [architecture checks](test/architecture.test.ts) for
  dependency rules, and behavior tests for semantic guarantees. Update the
  documentation and checks when deliberately changing a boundary.

Before finishing a change, ask:

> Can someone explain why this piece is correct using its own code and its
> dependencies' contracts? Does it know, access, or decide anything that belongs
> to another owner?

If answering requires inspecting unrelated internals, improve the boundary or
explain why that coupling is necessary.

## Making a change

Read the relevant [design decisions](docs/README.md), including later
amendments, the [architecture](docs/architecture.md), which says where a
change goes, and the existing implementation and tests. Follow the
established phases and reuse evaluation-order machinery. Verify uncertain
library behavior from local source.

1. **Start with a test.** Write a small Rust example, usually a corpus case
   in `test/corpus/`, and the JavaScript it should produce. For a bug, see
   the test fail before changing the compiler.
2. **Compare with native Rust** where the two should agree: a corpus case
   runs both and compares their output. Test intentional differences
   explicitly.
3. **Read the generated JavaScript.** It should read as a person would write
   it. Review every snapshot diff before accepting it with `bun run bless`.
4. **Prove the tests can fail.** For each new behavior or guard, add a
   mutation to its module's list in `scripts/mutations/`: a plausible bug
   that its tests must catch ([ADR 0093](docs/decisions/0093-mutations.md)).
   A mutation that survives means a missing test.
5. **Record the decision.** A new semantic choice gets a design decision in
   `docs/decisions/`, indexed in [docs/README.md](docs/README.md); a change
   to an existing one amends it, saying what changed and why. A difference
   from native Rust also goes in [the semantics](docs/semantics.md), with
   its section's other differences and in the list at its end.
6. **Track progress.** Update [roadmap](ROADMAP.md) items when their
   acceptance criteria are met, with evidence, and
   [the crate corpus](docs/crate-corpus.md) when a change moves it.

Keep native and browser compiler behavior consistent, and check real
examples or the playground when the change affects integration.

## Verification and review

Use the pinned Rust toolchain and Bun for JavaScript tooling. Commands are
in [package.json](package.json).

While working, run the checks that exercise the changed responsibility: its
corpus cases, its module's mutations, the architecture test. **Before
pushing, run everything [the check workflow](.github/workflows/check.yml)
runs, and rustc's tests**: a change can break what it never meant to touch,
and a reformatted line can break a mutation. [AGENTS.md](AGENTS.md) lists
the exact commands. On macOS, run them in its Linux VM, where the checks
that build native programs are many times faster; on Linux, run them
directly. Commit any list a check rewrites, such as rustc's known failures,
with the change that moves it. Documentation-only changes need content and
link checks.

Commit and pull request titles start with `feat(rust-js):`,
`fix(rust-js):`, `chore(rust-js):` or `perf(rust-js):`. In the pull
request, explain the problem, the resulting behavior, and how it was
verified. State any remaining limitations, and any checks that were not
run.
