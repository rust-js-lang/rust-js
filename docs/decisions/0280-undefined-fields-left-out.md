# 0280. A field that's `undefined` is no key

Status: Accepted. Amended: a `None` field a comment is written on keeps its
key, `undefined`, the comment's place.

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

## Amendment: wherever the object is written

The readability pass that leaves the field out skipped a template's
values, so the same `Options { value: None }` was `f({})` returned and
`f({ value: undefined })` in a `format!`, and `"value" in options` told
them apart. It now visits every expression an expression holds, named
variant by variant, so a new one is a compile error until it's handled.

## Amendment: a field a comment is written on keeps its key

React's `createCacheNode` writes each field's meaning after it:

```js
return {
  s: UNTERMINATED, // status, represents whether the cached computation returned a value or threw an error
  v: undefined, // value, either the cached result or an error, depending on s
  o: null, // object cache, a WeakMap where non-primitive arguments are stored
  p: null, // primitive cache, a regular Map where primitive arguments are stored.
};
```

The port's `v: Cell::new(None), // value, ..` was left out, and its
comment with it, though every comment is in the JS where its code is
(ADR 0444). A comment on its own lines before the field was worse: it
was the next field's, `// value ..` above `o: null`.

**A `None` field a comment is written on keeps its key, `v: undefined, //
value ..`**: one after its value on its line, in it, or on lines of their
own right before it. Only there is the comment where its code is: no other
place says what it says. A field without one is left out, as before.

- **Which fields have one is the struct literal's** (`aggregates.rs`,
  `commented_fields`), read from its text as rustc's lexer reads it, and
  carried on the object as written, `js::ObjectLayout::commented`, its keys,
  which the pass that leaves `undefined` out keeps: a field's value is a
  temporary until tidy reads it in place, so the object, not the value,
  says so.
- Case: A still: what reads the key reads `undefined` either way, and
  `$eq` and `$key` read a key of `undefined` and none alike. `in` and
  `Object.keys` see it, as they see React's.

Held by `test/lowering-comments.test.ts`, "a None field a comment is
written on keeps its key and its comment", run in Node, and the mutations
`commented-none-left-out` (`prepare.ts`), `commented-field-trailing-unread`
and `commented-field-leading-unread` (`lower/aggregates.ts`).
