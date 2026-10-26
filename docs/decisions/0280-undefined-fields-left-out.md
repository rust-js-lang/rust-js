# 0280. A field that's `undefined` is no key

Status: Accepted. Amended: a `None` field a comment is written after on
its line keeps its key, `undefined`, the comment's place; a comment before a
field changes no key, but where no key follows it.

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

Held by `test/lowering-comment-fields.test.ts`, "a None field a comment is
written on keeps its key and its comment", run in Node, and the mutations
`commented-none-left-out` (`prepare.ts`), `commented-field-trailing-unread`
and `commented-field-leading-unread` (`lower/aggregates.ts`).

## Amendment: a comment before a field speaks of the `None`s after it

Superseded the same day by the amendment below: it wrote keys upstream
leaves out.

React's `CacheContext` heads two fields with one comment:

```js
  // We'll initialize these at the root.
  _currentValue: null as any,
  _currentValue2: null as any,
  _threadCount: 0,
```

The port's `current_value: Cell::new(None), current_value2:
Cell::new(None)` under it was `currentValue: undefined` and no
`currentValue2`: the comment kept its own field's key and not its
sibling's, so the two `None`s it speaks of were written two ways.

**A comment on lines of their own before a field speaks of the `None`s
written right after that field too**, each on the line after the one
before it with no comment between: each keeps its key, `currentValue:
undefined, currentValue2: undefined`. Another field, a blank line or a
comment ends them; a comment after a value on its line is that field's
alone, as before.

- **Which are `None`s is MIR's**, a `None` or `Cell::new(None)` the literal
  writes (`Given::WrittenNone`, ADR 0810), handed to `commented_fields`
  with the literal's text, which says where its comments and blank lines
  are.
- An optional field's `None` is left out as before where no comment is on
  it or heads it (ADR 0810).
- Case: A still, as above.

Held by the same test, which adds a run ended by another field and one
ended by a blank line, and the mutations `commented-run-unread`,
`commented-run-past-a-value` and `commented-run-past-a-blank-line`.

## Amendment: a comment changes which keys are written only where it has no other place (2026-10-26)

The amendment above wrote keys React's committed files don't have.
createContext's object, `// These are circular` before `Provider` and
`Consumer`, gained `_currentRenderer: undefined, _currentRenderer2:
undefined`, which upstream sets only in DEV; ReactElement's, `// Record
the component responsible for creating this element.` before `_owner`,
gained `_store: undefined`, which upstream defines only in DEV, by
`Object.defineProperty`. Each is a key `Object.keys` sees, in production
too: a shape at the JS boundary that changed. And before it, a comment on
lines of their own before a `None` kept that one key and not its
sibling's, `_currentValue` and not `_currentValue2`, so a comment decided
what the object is.

**A comment changes which `None` keys are written only where it has no
other place.** A comment after a field's value on its line, or in it, is
that field's alone: it keeps its key, `v: undefined, // value ..`, as
before, as React's createCacheNode has it. A comment on lines of their own
before a field heads what follows: the keys are what they are without it,
and it's before the next key the object has, `// We'll initialize these at
the root.` before `_threadCount`, `_currentValue` and `_currentValue2` left
out alike. Only where no key follows does the `None` it's before keep its
key, the comment's place then, as it would otherwise be outside the
object.

- **Which comments are where is the literal's** (`aggregates.rs`,
  `commented_fields`): the keys a comment is on, `ObjectLayout::commented`,
  and the keys one is before, `ObjectLayout::led`; **which keys are left
  out is preparation's** (`src/prepare/undefined_keys.rs`), which alone
  sees each value as it ends, and keeps a led `None` only where no key
  after it is kept.
- **What this undoes**: the amendment above, and the first amendment's
  `None` a comment is before keeping its key, which it said put the
  comment "before the next field"; it does, and now that's the rule, as the
  comment heads what follows.
- Case A still: what reads a key reads `undefined` either way; `in` and
  `Object.keys` see what upstream's do.

Held by `test/lowering-comment-fields.test.ts`, "a comment before a field
changes no key but where it has no other place", of createContext's,
ReactElement's and the cache context's shapes and a comment before a last
`None`, run in Bun, and "a None field a comment is written on keeps its
key and its comment"; and the mutations `commented-field-leading-unread`,
`commented-field-leading-as-on` (`lower/aggregates.ts`),
`led-none-always-kept` and `led-none-last-left-out` (`prepare/undefined_keys.ts`).
