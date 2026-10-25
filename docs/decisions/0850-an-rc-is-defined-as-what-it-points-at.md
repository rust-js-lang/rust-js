# 0850. An `Rc` is `Defined` as what it points at, and a counted one given to JS is refused

Status: Accepted. Extends [0844](0844-a-library-that-reads-no-count-counts-no-rc.md),
whose uncounted `Rc` is the value it points at, to a binding's values.
Amended by [0884](0884-a-maps-value-is-any-value.md): a `WeakMap`'s and
a `Map`'s value is any value, so one of an `Rc` needn't be `Defined`; an
`Rc` still is where what it points at is.

Case ([0262](0262-when-rust-and-js-disagree.md)): N of an uncounted
`Rc`, the value itself, which JS shares as Rust does; D, refused, of a
counted one, whose count JS couldn't keep.

## Context

React's cache keeps its nodes in a `WeakMap` and a `Map`, each node
reached from several:

```js
cacheNode = createCacheNode();
objectCache.set(arg, cacheNode);
```

A port's node is an `Rc<CacheNode>`, shared as upstream shares the
object. `weak_map::new::<JsObject, Rc<CacheNode>>()` was E0277: a value
a `WeakMap` holds is `js::Defined`, never `undefined` nor `null`, as its
`get` gives `Some(v)` as `v` itself, and an `Rc` wasn't one, where a
`Box` of a `Defined` value is.

## Decision

**`Rc<T>` and `Arc<T>` are `js::Defined` where `T` is, as `Box<T>` is;
a binding call whose type arguments hold a counted `Rc` or `Weak`,
`WeakMap<K, Rc<Loud>>::get`, is refused.**

```rust
unsafe impl js::Defined for CacheNode {}

cache_node = create_cache_node();
object_cache.set(arg, Rc::clone(&cache_node));
```

```js
cacheNode = createCacheNode();
objectCache.set(arg, cacheNode);
```

- **An uncounted `Rc` is the value it points at** (ADRs 0023, 0844), a
  JS object a `WeakMap` holds and gives back: a clone of it, as JS shares
  it, which no count or destructor of the crate's could tell apart.
- **A counted one is refused where a binding is given or gives it**
  (`std_types::rc::check_counted_given_to_js`): JS holds its `{ value,
  strong, weak }`, and what it gives back, `get`'s `Rc`, is an owner Rust
  never counted, whose drop would drop what the map still holds. "a
  counted `Rc<Loud>` given to JS or taken from it .. an `Rc` is the value
  it points at where the crate reads no count and drops nothing it
  points at".

## Why

- **It's upstream's object**, shared by its map and its caller, as an
  `Rc` is in Rust.
- **Sound only where it isn't counted**, which the crate's one answer,
  `Counted::counts`, says.

## Alternatives

- **The node leaked, `&'static CacheNode`**: Rust bent for the compiler,
  a leak upstream hasn't.
- **A counted `Rc` given to JS as its box, its count raised by what JS
  gives back**: JS can't drop what it holds, so its count would never
  fall again; a destructor would never run.

## What holds it

- `test/js-builtins.test.ts`, "an uncounted Rc a WeakMap holds is the
  value, and a counted one is refused": a node set and read back, run,
  and a `Loud`'s `Rc`, refused by its message.
  `test/rest-parameters.test.ts`, React's cache nodes.
- Mutations in `scripts/mutations/lower/std_types/rc.ts`.
