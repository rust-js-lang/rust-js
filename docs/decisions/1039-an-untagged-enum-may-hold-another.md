# 1039. An untagged enum's variant may hold another untagged enum

Status: Accepted (rd305). Extends [0214](0214-untagged-enums.md) and
[1036](1036-an-untagged-enums-struct-is-told-by-a-member-it-declares.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the output's shape:
a value of the inner enum is one of its variants' payloads, which the
other variants' aren't, so the test tells each as Rust's discriminants do.

## Context

React's noop renderer's childToJSX takes `null | Instance | TextInstance
| Array<Instance | TextInstance>`, and tells the list first,
`Array.isArray(child)`, then an instance by its `children`. The port has
rd291's untagged `Child` of `Instance | TextInstance` and wrote `enum
JsxChild { Child(Child), Children(Rc<RefCell<Vec<Child>>>) }`, which
rust-js refused: "JS can't tell a `Child` from another variant's value".
So the port tagged `JsxChild`, `childToJSX({TAG: 'Child', _0: child[0]},
null)` and `child._0.children`, where upstream passes `child[0]` and reads
`child.children` (gap rd305, P2).

## Decision

**A variant may hold another untagged enum: its value is one of that
enum's variants' payloads, of each of their kinds** (`untagged::Kind::Union`).

- **Its test is those kinds' tests together**, each leaving out what the
  outer's other variants are more exactly, as a variant's does (ADR 0214):
  childToJSX's `Child` is an object no array is; a number or a word's
  `case "number": case "string":` of a `switch (typeof keys)` (ADR 0668).
  Where an object of them is tested whole, a struct of them told by a
  member, a class or a test function is that object already, so no test
  of its own: `typeof child === "object" && !Array.isArray(child)`, not
  `Array.isArray(child.children) || (..)`.
- **The inner enum's variant is then told as it is alone**: `JsxChild::
  Child(Child::Instance(instance))` is `Array.isArray(child.children)`
  once `Array.isArray(child)` fails, as upstream writes it.
- **A member another variant's struct has too tells nothing there** (ADR
  1036), of each of the inner's structs and of the outer's: a `Container`
  told by its `rootID` beside an item whose instance's `children` it has
  too makes the item an object no container is.
- **Two variants of a kind together are refused** as two of one kind are:
  a number or a word beside a number, or a container told by `children`
  beside an instance told by it.
- A test function an inner variant's type names, `isValidElement`, is
  imported where only the outer's pattern tests it.

Refused: an inner enum with an `#[rust_js::otherwise]` variant, a variant
that's `undefined` or `null`, or one that holds a third untagged enum in
turn, itself say, as JS can't tell what it is from what it's not, or it
would never end.

## Alternatives

- **Tag the outer enum**: what the port did, `{TAG: 'Child', _0: ..}`,
  where React passes the value itself.
- **Flatten the inner enum's variants into the outer**: the port's types
  would no longer be React's, an `Instance | TextInstance` the outer's
  callers name.

## Consequences

- The port's `JsxChild` is untagged: `childToJSX(child[0], null)`.
- Held by the corpus's `untagged_in_untagged`, against native Rust:
  childToJSX's `JsxChild`, a container beside an item of an instance or a
  number, a key of a number or a word; `test/jsx-types.test.ts`'s
  `isValidElement(nodes) || typeof nodes === "string"`, imported; and
  `test/untagged-members.test.ts`'s refusals. Mutations
  `untagged-in-untagged-refused`, `untagged-in-untagged-overlaps-none`,
  `untagged-in-untagged-member-kept`, `untagged-member-in-an-enum-unseen`,
  `untagged-in-untagged-in-untagged`, `untagged-in-untagged-member-tested`
  and `untagged-in-untagged-test-unimported` (`untagged.ts`); and
  `union-keeps-whole-exclusions`, `union-keeps-arrays`,
  `union-keeps-nulls` and `untagged-shared-member-telling`, their finds
  moved.
