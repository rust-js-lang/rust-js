# 0734. `extend` of an array written in place is `push` of its items

Status: Accepted.

Case ([0262](0262-when-rust-and-js-disagree.md)): it can't be observed:
`push(a, b)` makes `a` then `b`, then pushes each in order, as making the
array, then pushing each of it, does.

## Context

React's addObjectDiffToProperties pushes two rows at once:

```js
properties.push([REMOVED + line, '…'], [ADDED + line, '…']);
```

The port's `properties.extend([(REMOVED.to_string() + &line, ..), (ADDED
.., ..)])`, the idiom for more than one, printed the runtime's
`$extend(properties, [[..], [..]])`, a helper JS's own `push` does the
job of.

## Decision

**A `Vec`'s `extend` of an array written in place is `v.push(..items)`**,
its items as written (`combinators::extended`). Of an iterator, a slice
or a variable, and onto a set, a map, a deque or a heap, it's
`$extend`'s, as before: `push(...xs)` of what may be long would be
limited by JS's count of arguments.

## Consequences

- The port's two `extend`s read as upstream's `push`es.
