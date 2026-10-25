# 0875. A `js::Set` holds any value, and a `Set` and a `Map` are made of what they start with

Status: Accepted. Extends [0849](0849-a-map-takes-any-key.md), whose
`js::Map` keys any value, to a set of them; amends its `map::new()`,
which now takes what the map starts with, `None` of nothing. Amended by
[0884](0884-a-maps-value-is-any-value.md): a `Map`'s value is any value,
so BadMapPolyfill's port keys its frozen object to `null`, `Kind::Null`,
as upstream does.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: each is JS's own
collection, a value the one `SameValueZero` says, as TypeScript's `Set<T>`
and `Map<K, V>` are.

## Context

React's BadMapPolyfill asks whether the host's `Map` and `Set` take a
frozen key:

```js
const frozenObject = Object.freeze({});
new Map([[frozenObject, null]]);
new Set([frozenObject]);
```

The js crate had no `Set`, and its `map::new()` took nothing: TypeScript's
`new Map(entries?)` and `new Set(values?)` were narrower in Rust. A
`HashSet` keys by `Hash` and `Eq`, which no `&Unknown` and no `&JsObject`
has; one of the program's own types, by value or by itself, is a JS
`Set` already (ADRs 0059, 0864).

## Decision

**`js::Set<T>` is TypeScript's `Set<T>`, of any `T`: its `add`, `has`,
`delete`, `size` and `clear`. `js::set::new(values)` and
`js::map::new(entries)` take what each starts with, as TypeScript's
constructors take an optional array, `None` of nothing.**

```rust
let frozen = js::object::freeze(js::dict::new::<u32>());
let map: &Map<Dict<u32>, bool> = js::map::new(Some(&[(frozen, true)]));
let set: &Set<Dict<u32>> = js::set::new(Some(&[frozen]));
let empty: &Set<Option<&'static Unknown>> = js::set::new(None);
```

```js
const frozen = Object.freeze({});
const map = new Map([[frozen, true]]);
const set = new Set([frozen]);
const empty = new Set();
```

- **One way per job, as 0849's maps**: a set a program keeps of what Rust
  hashes, its own types by value or by themselves, is a `HashSet`, a JS
  `Set` too, which native Rust runs; a `js::Set` is JS's, to share with JS
  or to hold what no `Hash` has, a value of any shape.
- **Its optional argument is an `Option` of its one function** (the
  architecture test "a binding's optional argument is an Option of its
  one function"): `map::new()` is `map::new(None)`, every caller's.

## Why

- **TypeScript's types**: `Set<T>` of any `T`, and the constructors'
  optional arrays, which upstream writes.
- **No `Hash` keys a value of any shape**: JS's `SameValueZero` isn't an
  `Eq`, `NaN` itself, an object by itself.

## Alternatives

- **A `HashSet<&JsObject>`**: `JsObject` has no `Eq` of its identity, and
  a set of any value, `Option<&Unknown>`, couldn't be one.
- **A second constructor, `map::from_entries(..)`**: a second binding of
  `new Map`, which the architecture test refuses.

## Consequences

- A map whose value is `null`, upstream's `[[frozenObject, null]]`, has no
  Rust type: a `Map`'s value is `Defined`, never nullish, as its `get` is
  an `Option` of it; BadMapPolyfill's port gives the map a value of its own.
  (Amended by 0884: a `Map`'s value is any value, `get` of an `Option`
  told by `has`, and upstream's `null` is `Kind::Null`.)

## What holds it

- `test/js-collections.test.ts`, "a Set holds any value, and a Set and a
  Map are made of what they start with": React's frozen key in each, run;
  a set of an object twice, another, `undefined`, `null`, `NaN` twice, `0`
  and `-0`, six values; one of text; and the `.d.ts`'s `Set<string>`.
- 0849's tests, of `map::new(None)`.
