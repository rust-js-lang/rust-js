# 0884. A `js::Map`'s value is any value, and `get` tells a `None` from a missing key

Status: Accepted. Amends [0849](0849-a-map-takes-any-key.md) and
[0875](0875-a-js-set-holds-any-value.md), whose `Map`'s and `WeakMap`'s
value was `js::Defined`, and extends [0225](0225-unknown-values.md)'s
rule of a `Dict<Option<_>>`'s `get` to them. Amends
[0850](0850-an-rc-is-defined-as-what-it-points-at.md), whose `Rc` a
`WeakMap` held had to be `Defined`.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: a value is stored as
Rust's is everywhere, and `get` gives what Rust's would, `Some(None)` of a
key whose value is `None` and `None` of a key the map lacks; nothing runs
differently.

## Context

React's BadMapPolyfill asks whether the host's `Map` takes a frozen key:

```js
const frozenObject = Object.freeze({});
new Map([[frozenObject, null]]);
new Set([frozenObject]);
```

A `js::Map<K, V>`'s `V` was `js::Defined`, never `undefined` nor `null`,
because its `get` was `map.get(key)` as it is, an `Option<V>` whose
`undefined` is `None`: of a value that's `undefined` itself, a `None`, the
`Some(None)` Rust has would read as `None`. So the port couldn't write
upstream's `null`, neither `None::<&Unknown>` nor `Kind::Null`, which
holds `undefined` and `null` and so isn't `Defined`; ADR 0875's port gave
the map a value of its own, `true`. TypeScript's `Map<K, V>` takes any
`V`: the binding was narrower than the original.

A `Dict<T>` takes any `T` already (ADR 0225). Its `dict::get` of a
`Dict<Option<_>>` is `$dictGet(d, k)`, `Object.hasOwn(d, k) ?
$some(d[k]) : undefined`: `Some(None)` of a JSON `null`, `None` of a key
that isn't its own, as `d[k]` alone couldn't tell. A `HashMap<K,
Option<V>>`'s `get` is the runtime's `$someGet(m, k)`, `m.has(k) ?
$some(m.get(k)) : undefined`, for the same reason. Neither stores a `None`
as `null`: `dict::set(d, "a", None)` is `d.a = undefined`, and
`m.insert(1, None)` is `m.set(1)`.

## Decision

**A `js::Map`'s and a `js::WeakMap`'s value is any value, as a `Dict`'s
is. A binding's keyed read whose result is an `Option` of what may look
like `None`, whose `Some` is boxed (ADR 0051), tells a key whose value is
`None` from a key that isn't there: a method `"get"`, a JS `Map`'s, by
`has`, `$someGet(map, key)`, as a `HashMap`'s is; `"get []"`, a `Dict`'s,
by its own key, `$dictGet(d, key)`, as before.** A `None` is `undefined`,
as it is everywhere; JS's `null` is `Kind::Null`, as `function::apply`'s
`this` is (ADR 0848):

```rust
let frozen_object = js::object::freeze(js::dict::new::<Kind>());
js::map::new(Some(&[(frozen_object, Kind::Null)]));
js::set::new(Some(&[frozen_object]));

let counts: &Map<str, Option<u32>> = js::map::new(None);
counts.set("later", None);
match counts.get(key) {
    Some(Some(n)) => ..,
    Some(None) => ..,
    None => ..,
}
```

```js
const frozenObject = Object.freeze({});
new Map([[frozenObject, null]]);
new Set([frozenObject]);

const counts = new Map();
counts.set("later", undefined);
const get = $someGet(counts, key);
```

- **One rule, of a keyed read's result** (`binding_calls.rs`'s
  `gives_boxed_option`): an `Option` of an `Option`, a `()`, a `Kind` or a
  type parameter, which `boxed_payload` says may look like `None`. A value
  that never does, `Map<str, u32>`'s, is `map.get(key)` as before. Which
  read is a `Map`'s is the binding's own form, a method whose JS is `get`
  (`bindings.rs`'s `reads_a_key`), the js crate's `Map::get` and
  `WeakMap::get` and a binding of a program's own of the same JS.
- **`has`, `set`, `delete` and iteration are as they were**: what's set is
  the value as Rust has it, `undefined` of `None`, which `has` says is
  there and a callback of `forEach` is given as `None`. The `.d.ts` of
  a `Map<str, Option<&Unknown>>` is `Map<string, unknown | null |
  undefined>`, as any `Option`'s value is declared (ADR 0876), so a JS
  caller's `null` is a `None` too, and `get` reads it `Some(None)`.
- **A `Set` needs nothing**: its `T` is any value already (ADR 0875), and
  it has no `get`.
- **`js::Defined` stays** where an `Option` of a value must be the value
  itself, a structured clone's (ADR 0225) and `JsonText`'s; an `Rc` is
  still one where what it points at is (ADR 0850), which a `WeakMap`'s
  value no longer asks.

## Why

- **TypeScript's `Map<K, V>`**, of any `V`, and upstream's own JS,
  `new Map([[frozenObject, null]])`, with nothing of the port's own.
- **It's the rule a `Dict`'s and a `HashMap`'s `get` already have**, with
  the runtime's helper a `HashMap`'s uses; nothing new decides what a
  `None` is.

## Alternatives

- **A map's `None` stored as `null`**, `#[rust_js::nullable]`'s storage
  (ADR 0275) of the value a `set` and a `new Map(entries)` are given: a
  second rule of what a collection's `None` is, which no `Dict`, `HashMap`
  or `Set` has, and one `new Map([[k, None]])` couldn't follow where
  the entries are an array made before the call, without
  `entries.map(..)` of its own. `get` would need `has` either way, as a
  JS caller may set `undefined`. Upstream's `null` is a value of its own,
  `Kind::Null`, which the port writes as upstream does.
- **`V: Defined` kept, and a second binding of `Option` values**: two
  maps of one JS `Map`.

## Consequences

- A `Map` or `WeakMap` of a value the program's own generic code keeps,
  `Map<K, V>` of a `V` it's given, reads `$someGet(map, key)` too, as a
  `HashMap<K, V>`'s does there, where it was `map.get(key)`: a `V` may be
  an `Option` now.
- `unsafe impl js::Defined for CacheNode {}` isn't needed for a `WeakMap`
  of `Rc<CacheNode>` any more; it's harmless where it's kept.

## What holds it

- `test/js-collections.test.ts`, "a Map's value is any value, and get
  tells a nullish one from a missing key": BadMapPolyfill's port, `new
  Map([[frozenObject, null]]);`, a `Map<str, Option<u32>>` made of `None`
  and `Some`, set, read with `get` and `has`, a map JS made of `undefined`,
  `null` and `0`, a `WeakMap`'s, and the `.d.ts`; run.
- Corpus `map_option_values` against native Rust, a binding of a program's
  own `Map`: `Option`, `Option<&str>`, `Option<Option<u32>>` and `()`
  values set, read, deleted and stepped with `forEach`, and a function
  generic over the value.
- The guide's Collections (types.md), BadMapPolyfill and a match of
  `get`.
- Mutations `map-get-of-options-plain` and `keyed-get-of-any-value`
  (`lower/binding_calls.ts`); `dict-get-of-options-plain` still holds a
  `Dict`'s.
- `docs/readability.txt` blessed, its renames' share 717.3 to 717.6 in
  10,000 lines: the corpus case is new JS, whose 11 renames, `value$1` to
  `value$5`, are beside a `value` its `{:?}` of an `Option` names, each
  format arrow's parameter, and an `if let Some(value)` of a boxed `Some`,
  whose box is `value$1`, as a `HashMap`'s `$someGet` is; no JS that was
  there before reads differently.
