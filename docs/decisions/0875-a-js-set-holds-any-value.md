# 0875. A `js::Set` holds any value, and a `Set` and a `Map` are made of what they start with

Status: Accepted. Extends [0849](0849-a-map-takes-any-key.md), whose
`js::Map` keys any value, to a set of them; amends its `map::new()`,
which now takes what the map starts with, `None` of nothing. Amended by
[0884](0884-a-maps-value-is-any-value.md): a `Map`'s value is any value,
so BadMapPolyfill's port keys its frozen object to `null`, `Kind::Null`,
as upstream does. Amended below: a `Set`'s `forEach`, `values`, `keys` and
`entries`, and a keyed collection's own methods run nothing.

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

## Amendment: a set's `forEach` and iterators, and what its methods run

React's ReactFiberLane collects the transitions of each lane:

```js
const transitions = root.transitionLanes[index];
if (transitions !== null) {
  transitions.forEach(transition => {
    transitionsForLanes.push(transition);
  });
}
```

and adds one while it holds the lane's table, `transitionLanesMap[index]`
of `root.transitionLanes`. A `js::Set` had neither `forEach` nor a way to
step it, and a borrow held across `transitions.add(transition)` was
counted: a binding may run anything (ADR 0863), and every borrow of the
table's type was checked, `$borrowMut` in a `try`/`finally`.

- **`set.for_each(f)` is `set.forEach(f)`**, as TypeScript's `Set<T>`
  has it, `f` given each value, `&'static T`, as JS holds it. JS runs `f`
  before `forEach` returns and keeps nothing of it, so `f` is an
  `impl FnMut` of any lifetime and may borrow what's around it, as
  `promise::new`'s executor may (ADR 0782): `|transition|
  transitions_for_lanes.push(transition)`. TypeScript's other parameters,
  the value again and the set, are left out, as a typed array's callback
  leaves out the array (ADR 0283).
- **`values()`, `keys()` and `entries()` are JS iterators** (ADR 0140), as a
  typed array's are: `for v in set.values()`, `for (k, v) in set.entries()`,
  `[v, v]` of each. No `IntoIterator`, as a typed array has none: the
  set's own iterator is `values()`.
- **A `Map`'s, a `Set`'s, a `WeakMap`'s and a `WeakSet`'s own methods and
  constructors run nothing**, `#[rust_js::runs_nothing]` (ADR 0867):
  `SameValueZero` calls nothing of a key or a value, and `new Set(values)`
  and `new Map(entries)` read the array they're given, not what's in it.
  So a borrow held across `transitions.add(transition)` isn't counted,
  and the lane's table is the array it holds. `forEach` runs what it's
  given, and isn't marked.

```js
export function addTransition(root, transition, index) {
  const transitionLanesMap = root.transitionLanes;
  const transitions = $index(transitionLanesMap, index) ?? new Set();
  transitions.add(transition);
  transitionLanesMap[index] = transitions;
}
```

Held by `test/js-collections.test.ts`, "a Set's forEach and iterators, and
a borrow held across its own methods": ReactFiberLane's
`addTransitionToLanesMap` and `getTransitionsForLanes`, run, no borrow
counted, and a set's values, keys and entries stepped.

