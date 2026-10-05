# Contributing to rust-js

**Rust in. Readable JavaScript out.** Keep compiler code simple, preserve
Rust's checks, and treat generated JavaScript as part of the product.
These guidelines apply to contributors, maintainers, and AI coding agents.

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
- **Keep dependency direction explicit.** Follow the compiler phases and
  permitted dependencies in [the architecture](docs/architecture.md). Put
  coordination in the piece responsible for orchestration. Avoid cycles
  between responsibilities.
- **Carry decisions forward as explicit data.** A later phase consumes an
  earlier phase's answer rather than reconstructing it from names, generated
  text, or implementation details.
- **Keep boundaries small and meaningful.** Introduce an abstraction when it
  hides a real responsibility or protects an invariant. Avoid wrappers,
  forwarding APIs, and frameworks that merely move code around.
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
amendments, and the existing implementation and tests. Follow the established
phases and reuse evaluation-order machinery. Verify uncertain library behavior
from local source.

Start compiler changes with a small Rust example and the JavaScript it should
produce. Compare behavior with native Rust where they should agree. Document
and test intentional differences. Inspect generated JavaScript and review
snapshot diffs before accepting them. Unsupported features must produce useful
errors; never bypass rustc's checks or publish partial output on failure.

Document new semantic choices and limitations. Update relevant
[roadmap](ROADMAP.md) items when their acceptance criteria are met, with
evidence. Keep native and browser compiler behavior consistent, and check real
examples or the playground when the change affects integration.

## Verification and review

Use the pinned Rust toolchain and Bun for JavaScript tooling. Run checks in
the Linux VM through `scripts/linux-vm.sh`; [AGENTS.md](AGENTS.md) contains
the VM setup and required checks before pushing. Available commands are in
[package.json](package.json), and CI checks are in
[the check workflow](.github/workflows/check.yml).

Choose checks that exercise the changed responsibility and its contracts.
Documentation-only changes need content and link checks. Explain the problem,
the resulting behavior, and how it was verified in the pull request. State any
remaining limitations or checks that were not run.
