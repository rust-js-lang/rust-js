# 1162. An untagged variant told by its key is `#[rust_js::test(key)]`, JS's `in`

Status: Accepted (2026-11-15), the lead's decision (#1846). Extends
[1036](1036-an-untagged-enums-struct-is-told-by-a-member-it-declares.md).

Case: third ([0262](0262-when-rust-and-js-disagree.md)), results change:
a Proxy's traps and a getter tell a read from a membership test.

## Context

An untagged enum's struct variant told apart by a member it declares,
a bare `#[rust_js::test]` on its field, is told by that member's kind,
`Array.isArray(child.children)`, `typeof move.bound === "string"`, which
reads it (ADR 1036). A faithful port of TypeScript's `in` narrowing,
`"bound" in move ? move.bound : ..` (Codex's numeric step function), reads
nothing to tell: a Proxy's `has` trap, no `get`, and no getter run; it
counts an inherited key too. No form wrote it.

## Decision

**`#[rust_js::test(key)]` on a variant's struct's field tells it by its
key alone**: an object that has it, `"bound" in move`, its value unread
(`untagged::Kind::Key`, the one classifier, `untagged_kind` and
`kind_test`). After its object test where the union holds what isn't an
object, as `in` throws of a primitive, `typeof given === "number"` first;
of key-tested variants and an `otherwise`, each key in the order its
variants are written. Two variants of one key are refused, as two of one
kind are; a field's `test(..)` of another word is refused, naming the two
forms.

Not a switch by itself where a key is unique: the two tests may tell one
object apart otherwise. `{ bound: 5 }` has a `bound`, a `Bound` to `in`,
and its `bound` isn't a string, a `Direction` to a kind test of a
`String` field. So they're two jobs, and each field says which it is
(the lead's decision).

## Consequences

- test/untagged-keys.test.ts: a Proxy's traps, `has bound` then `get
  bound` in its arm alone, `has bound` then `get direction` of the other;
  a throwing getter never read; an inherited key; `{ bound: 5 }` told
  apart otherwise by key and by kind; a number variant beside, no `in` of
  it; the refusals. Mutations in `untagged.ts` and `bindings/validated.ts`.
- Guide: interop/types.md's untagged enums, interop/bindings.md's
  attribute table. Bare `#[rust_js::test]`'s users, 9 in the corpus and
  7 in the React port, are as they were.
