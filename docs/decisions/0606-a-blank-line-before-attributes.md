# 0606. A blank line before an item's attributes is before the item

Status: Accepted. Extends [0444](0444-a-comment-is-where-its-code-is.md)'s
blank lines.

Case ([0262](0262-when-rust-and-js-disagree.md)): A: layout only.

## Context

A blank line in the Rust before a statement is one in the JS (ADR 0444).
An item's span starts at the item, past its attributes, so the blank line
before `#[cfg_attr(rust_js, rust_js::name = "..")]` wasn't seen: React's
SchedulerFeatureFlags lost the blank lines between its groups of flags,
and a `#[allow(deprecated)] let ..` its own.

## Decision

**The lines of an item's attributes, `#[..]`, are its own**: a blank line
before them is before it.

## What holds it

- `test/lowering.test.ts`, "a blank line before an item's attributes is
  kept"; a corpus case's blank line before `#[allow(deprecated)]`.
- Mutation: the attributes' lines not skipped.
