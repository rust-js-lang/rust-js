# 0277. A variable shadowed by its own value is the variable

Status: Accepted.

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

Rust shadows a variable with what's made of it, and every local of a JS
function has a name of its own, so react.dev's errors page read:

```rust
let Type = ElementType::from_unknown(Type);
jsx! { <Type key={key} {...props} /> }
```

```js
const Type$1 = Type;
return <Type$1 key={key} {...props} />;
```

and `let content = content.clone();` before a closure was `const content$1
= content;`. Its original has `<Type ..>` and `content`.

## Decision

**`const n$1 = n;`, a variable shadowed by its own value, is `n`**: each
read of `n$1` reads `n`, and the `const` is gone. Only where `n` holds that
value wherever `n$1` is read: nothing sets `n` after it, nor in a closure,
nor in a loop it's in, where a closure of each turn keeps its own. One of
another name, `let kept = text;`, keeps the name written.

## Why

- **No program can tell**: the two hold one value, the same object or the
  same string, wherever the new one is read.
- **It's the JS a person writes**, and the name the Rust has.
- **It's tested**: a compiler test shadows a captured string and a number
  set before, and keeps one of another name and one a loop sets again,
  whose closures each read their own turn's; mutations alias each.
