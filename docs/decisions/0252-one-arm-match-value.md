# 0252. A one-armed match's value is its body's

Status: Accepted.

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

`jsx!` captures a component's props in a one-armed match where its `key`
comes first, to keep JSX's order: `match (a, b) { (x, y) => .. }`.
rust-js lowered a match whose value is used as any other, into a
variable each arm sets, so react.dev's PackageImport, which keys each
CodeBlock, was:

```js
let tmp;
tmp = <CodeBlock {...props} isFromPackageImport noMargin noMarkers key={i} />;
return tmp;
```

## Decision

**A match of one arm, which rustc has checked always matches, binds its
pattern as a `let` would, and its value is its body's.**

```js
return <CodeBlock {...props} isFromPackageImport noMargin noMarkers key={i} />;
```

- **A part of its subject that reads only variables that never change is
  read where it's used**, as jsx!'s captured `ref={Some(anchor)}` is,
  `ref={anchor}`, not `const match = anchor`. (Amended.)

## Why

- **It's the JS a person writes.**
- **It runs the same**: the subject is made, and its parts bound, before
  the body, as before; only no variable holds the body's value.
- **It's tested**: a JSX test keys a component first, beside its props
  spread, in an `if` that gives it back, and the Next.js build test a
  link of a ref; the corpus runs as before. Mutations hold the value in a
  variable again, and the ref in a `const`.
