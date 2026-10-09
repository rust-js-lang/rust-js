# 0309. A `match` that only gives a value is one expression

Status: Accepted. Extends the two-arm `match` as a conditional, and
[0298](0298-truthy-options.md)'s tests.

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A `match` of a call whose arm binds, `match path.rsplit_once('/') {
Some((_, file)) => file, None => path }`, was a variable set in an `if`,
as the playground's app had it three times:

```js
let file;
const match$1 = $rsplitOnce(path, "/");
if (match$1) {
  file = match$1[1];
} else {
  file = path;
}
```

What an arm binds was named in place only of a place, so a call's value
took the statements. And `map_or` of the same option was another
expression, `option != null ? option[1] : path`: two forms for one job.

## Decision

**A `match` of two arms that only give a value is one expression: of a
call's value, through a `const` of it; an option's value or a default,
`e ?? d`; a part of it that's never `null` or `undefined`, `e?.[1] ?? d`;
and `map_or` is the same.**

```rust
let current = match project.current_state() {
    Some(state) => state,
    None => blank,
};
let text = match files.iter().find(|(path, _)| path == shown) {
    Some((_, text)) => text.clone(),
    None => String::new(),
};
let n = match path.find('x') {
    Some(i) => i + 1,
    None => 0,
};
```

```js
const current = Project.currentState(project) ?? blank;
const text = files.find(([path]) => path === shown)?.[1] ?? "";
const match = $find(path, "x");
const n = match != null ? (match + 1) >>> 0 : 0;
```

- The `const` is of a value, a call's; a place a `match` can't name in
  place, one it copies, keeps its statements.
- `e ?? d` and `e?.[1] ?? d` read the option once, so they need no
  `const`, of a value or of a variable. A part that may itself be `None`,
  `()` or a generic's, isn't `?? d`'s: `d` would replace it.

## Why

- **It's the JavaScript a person writes**: one line, no variable set in
  branches.
- **It's the same program**: `e ?? d` is `e` where it isn't `None`, and
  `e?.[1]` ends where it is; a part never `null` or `undefined` is then
  `?? d`'s only where `e` was `None`.
- **One spell for one job**: a `match` and `map_or` of an option are one
  expression.
- **It's tested**: a lowering test runs each form, of a call, a variable
  and `map_or`, and a part that may be `None`; mutations keep the
  statements, the `const`, the ternary, `map_or`'s, print `?.[1]` as
  `[1]`, and take a nullable part as `?? d`'s.
