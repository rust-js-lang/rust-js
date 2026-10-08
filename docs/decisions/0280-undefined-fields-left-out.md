# 0280. A field that's `undefined` is no key

Status: Accepted.

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A struct is an object of each of its fields (ADR 0020), so one made with a
`None` has a key of `undefined`:

```js
meta={{ title, titleForTitleTag: undefined, version: undefined, description: undefined }}
const init = { method: "POST", headers: undefined, body, referrer: undefined, /* .. */ };
```

Hand-written JS leaves those keys out, as react.dev's pages do, and as
Sandpack's files are `{ code, hidden, active }`.

## Decision

**An object's field whose value is `undefined`, a `None`, `()` or a unit
struct, is no key**: `{ title }`, `{ method: "POST", body }`. Not in an
object with a spread, `{ ...flags, on: undefined }`, where the key
overrides the spread's.

The runtime reads objects alike either way: `$eq` compares the keys either
has, and `$key`, a set's or a map's, leaves out a field that's `None`.

## Why

- **No program can tell**: what reads the field reads `undefined`, which
  is its `None`, as JS reads a key that isn't there; `==` and hashing treat
  the two alike.
- **JS then does what the original does**: `in`, `Object.keys` and a spread
  of it, `{ ...defaults, ...props }`, see no key, where `undefined` would
  override a default.
- **It's tested**: a test makes a struct of a literal `None`, a default, and
  one given `None`, compares and hashes them alike, and keeps a spread's
  `undefined`; mutations keep the keys, drop the spread's, and count keys
  in `$eq` and `$key`.
