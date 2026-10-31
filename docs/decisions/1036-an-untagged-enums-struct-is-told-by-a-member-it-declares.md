# 1036. An untagged enum's struct is told apart by a member it declares

Status: Accepted (rd291). Extends [0214](0214-untagged-enums.md): two
structs, each an object, in one untagged enum.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the output's shape:
each value the program makes of a variant has its member, and the others'
objects don't, so the test tells each as Rust's discriminant does.

## Context

React's noop renderer types each host child `Instance | TextInstance`, two
plain objects its host config makes, and tells them apart by a member:
`if (isArray(child.children)) { .. } return textInstance.text;`. Its
parents are `Container | Instance`, told by `typeof parentInstance.rootID
=== 'string'`. The Rust is an untagged enum of the two structs, which
rust-js refused: "`Text` and `Instance` both hold an object, which JS can't
tell apart" (ADR 0214), so the port would have merged them into one struct
of optional fields, looser than Flow's types (gap rd291, P1: it held
createReactNoop, which 79 of React's test files use).

## Decision

**A struct's field marked `#[rust_js::test]`, bare, tells the struct apart
in an untagged enum by its kind**: the struct is an object whose member,
by its JS key, is of that kind, `Array.isArray(child.children)` of a
`Vec`, `typeof parent.rootID === "string"` of a `String`
(`untagged::Kind::Member`). The same attribute names a JS object type's
test function, `#[rust_js::test = "react#isValidElement"]`; on a field,
with no value, the member is the test.

- **A member another variant's struct has too tells nothing there**: that
  variant is an object, what the others aren't. React's `Container |
  Instance`: an `Instance` is told where `rootID` isn't a string, as a
  `Container` has `children` too.
- **Of an enum each variant of which is a struct, a plain one's test is
  only what the others aren't**, `!Array.isArray(child.children)`, not
  `typeof child === "object" && ..`, which holds of each.
- Arms alike in JS read the same member of each, `child.parent` (ADR
  1032), as before.

Refused: a member of a kind JS can't tell from an object, an object, an
`Option` or a struct, naming what it may hold.

## Alternatives

- **A JS expression in a string**, `#[rust_js::test = "Array.isArray(
  children)"]`: JS no rustc checks, where the field's name and type say
  the test.
- **`'children' in child`**: a member's presence, which a struct's
  `None`, left out (ADR 0280), would answer otherwise.
- **One struct of optional fields**: what the port would bend its types
  to.

## Consequences

- The port writes `#[cfg_attr(rust_js, rust_js::test)]` on `Instance`'s
  `children` and `Container`'s `root_id`, and the enums untagged.
- Held by `test/untagged-members.test.ts` (a test, alike arms, and the
  refusal) and the corpus's `untagged_told_by_members`, against native
  Rust: `Instance | TextInstance` and `Instance | Container`, each matched.
  Mutations `untagged-member-unread`, `untagged-member-tested-whole`,
  `untagged-objects-typeof-kept`, `untagged-shared-member-telling`,
  `untagged-member-left-in` and `untagged-member-of-an-object`
  (`untagged.ts`); `untagged-object-takes-arrays` amended.
