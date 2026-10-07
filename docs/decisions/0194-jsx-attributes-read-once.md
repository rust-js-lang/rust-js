# 0194. An attribute read before a child is read once

Status: Accepted. Amends [0040](0040-jsx.md).

## Context

JSX prints an element's attributes before its children, and rust-js keeps
Rust's order of reading them: where a child needs a statement of its own,
`const title = props.title;` of `{props.title.map(..)}`, the attributes
before it are read first, each into a `const` (ADR 0040). One read so
already, for an attribute before another that needed it, was copied again,
as react.dev's `IconCanary` showed, ported:

```js
const className = props.className;
const className$1 = className;
```

## Decision

**An attribute or a child already a `const`, or a variable nothing writes
again, is read as it is**, as a value a hook or a closure captures is: it
reads the same wherever it's read. So is a field of one of plain Rust
data, `p.size`, and a comparison, a conditional, `&&` or `||` of what
reads alike: `width={p.size === "S" ? "12px" : "20px"}`, `todos.length ===
0`. Not one whose type has a `&mut`, a raw pointer, interior mutability
(a `Cell`, an `Rc<RefCell<..>>`) or a JS object: what's done meanwhile
changes it, a getter's `n.textContent` too. `p.title.map(|t| ..)`, which
reads its option twice, reads such a field as it is too, where it was
`const t = p.title`. (Amended: a field read went in a `const` first.)

**A value read first is named as what it's read into is**: an attribute's
`const className`, a struct's field's `const href`, where a field made
before a later one's statements was `tmp`; of a tuple struct's, `0`, no
JS name, `tmp`. (Amended.)

**Only a child that does more than read, and whose JS needs a statement,
reads the attributes before it first**, its statements lowered aside and
looked at: one without, a `<Link>` whose props flatten an anchor's in
react.dev's `Breadcrumbs`, is read where JSX reads it, after them, as Rust
reads it, where a child Rust's expression made look complex read `key`
into a `const`; and one that only reads, `IconCanary`'s `const title =
props.title`, leaves them in place, as reads in either order are alike.

## Why

- **It's the JS a person writes**, and it's exact: a `const` is the same
  value later, so the order of reads is kept. A JSX test of `IconCanary`'s
  shape renders it, and finds no `const x$1 = x;`.
