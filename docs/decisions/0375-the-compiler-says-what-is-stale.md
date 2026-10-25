# 0375. What a build leaves stale is the compiler's to say

Status: Accepted. Amends [0091](0091-build-history.md) and
[0084](0084-owned-phases-and-host-boundaries.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)): which files a build
removes is the same; only who decides changes.

## Context

Which files an older build wrote that a new one may remove (ADR 0091) was
decided twice: by the native compiler, `output.rs`, and by the WASI host's
publisher, `tooling/publish.js`, "by the native compiler's rules", each its
own extension list, output directory and set of what's read. A test ran
every case against both, but the rule had two places, and so did the check
that an older manifest is this compilation's. AGENTS.md: one place answers
each question.

## Decision

- **The compiler decides, `output.rs`**: of an older build's manifest, the
  generated files in the output's directory that this build neither writes
  nor reads, each with the hash it was written with.
- **A publisher removes each still as written**: the native one,
  `publish.rs`, and a WASI host's, `publishArtifacts`, which also leaves a
  file that is, on the disk it has, one this build reads or writes by
  another name, a link to one: a WASI compiler sees no links.
- **A WASI compiler is given the older manifest**, by its host, of the
  compiler's own paths, and tells the host what's stale, its manifest's
  `stale`; the host removes them, and writes the manifest without it. A
  published manifest never has one: a build over an older one leaves what a
  build from nothing does.
- **Whether an older manifest is this compilation's** is the compiler's to
  say too; the host's check of it is gone.

## Consequences

- `publication.test.ts`'s WASI host cases are given the compiler's answer,
  as each native case has it; `host-boundaries.test.ts` holds that a link to
  what's written is kept.

## Amendment: a host never writes through a link to what the build reads

The WASI host's `publishArtifacts` (`tooling/publish.js`) checked each
artifact's hash but not where it wrote. An output `main.js` that was a
link to `main.rs` had its JavaScript written over the Rust source, and so
did a manifest path linked to an input. The native compiler already
refuses this as an output collision. The host now refuses it too,
before any file changes (`refuseOverlaps`). It refuses a write whose
resolved path is the build's input or one of its sources, and two writes
that resolve to one file. It mirrors the native rule across the host
boundary, which `test/publication.test.ts`'s "publication refuses a
write that resolves to one of the build's inputs" holds. Found by a
review on 2026-10-25.
