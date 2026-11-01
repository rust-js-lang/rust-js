# 1046. A JS collection keeps what it's given only in itself

Status: Accepted (the user's decision, 2026-11-01, issue #445). Extends
[0867](0867-a-binding-that-runs-nothing-and-aggregate-error.md) (a binding
that runs nothing) to where what a binding is given goes. The analysis that
reads it is [0863](0863-a-refcell-of-an-object-is-the-object.md)'s,
amended (a held borrow counts only the cells that can be its own), with
[0960](0960-a-dyn-is-any-type-only-where-made-in-place.md)'s rule kept for
cells that cross the crate.

Case ([0262](0262-when-rust-and-js-disagree.md)): A. The mark says what
JS's built-in `Map`, `Set`, `WeakMap` and `WeakSet` do: a value given to
`map.set(key, value)` is kept by the map and given back only by its
`get`, its iteration, and the like. Saying so changes no run. What it lets
rust-js prove, that a cell kept in one is no cell that code it can't see
reaches, only leaves out checks that can't fail.

## Context

React DOM's `hydrateHoistable` keeps each document's tag caches in JS
`Map`s, upstream's `tagCaches`, and holds a cache's array while it reads a
prop's getter (issue #445, rd201):

```rust
type KeyedTagCache = Map<str, Option<Rc<RefCell<Vec<&'static Element>>>>>;
..
cache.set(&key, Some(Rc::new(RefCell::new(vec![node]))));
..
let maybe_nodes = cache.get(&key).flatten();
if let Some(maybe_nodes) = maybe_nodes {
    let mut nodes = maybe_nodes.borrow_mut();
    .. js::get(props, "href") ..
```

A getter is code rust-js can't see, which may call back into the crate,
so that borrow is checked. To leave unchecked a cell of the same type that
no getter can reach, Fizz's `Batch`, rust-js has to know which cells a
borrow can be of: where each comes from and where it goes (ADR 0863,
amended). A cell given to a binding goes to code rust-js can't see, so it
may come back as any cell of its type. That includes one kept in a `Map`:
`map.set` and `map.get` are bindings.

`#[rust_js::runs_nothing]` (ADR 0867) says a binding runs no code. It
doesn't say where what it's given goes. Most such bindings keep it where
other code reaches it: `el.set_onclick(handler)` keeps the handler for the
browser to call, `Reflect.set(globalThis, k, v)` keeps `v` where any
script reads it. A `Map`'s `set` keeps what it's given only in the map.

## Decision

**A binding marked `#[rust_js::keeps_in_collection]` keeps what it's
given only in its collection, and gives only what that collection
holds.** Its collection is the JS object its `&self` is, or, of a binding
with no receiver, the one it makes and gives, `map::new(..)`'s. Nothing
else it's given, nor anything it gives, reaches code but the program's own
through it. So a value kept in a collection that no code rust-js can't see
reaches is no value that code can reach, as one kept in a `HashMap` isn't.
The analysis reads such a call as std's code is read (ADR 0863, amended):
what the collection holds of each of its type's parameters is one with
each value of that type the call is given or gives.

- **The js crate marks its collections' methods**: `Map`'s `get`, `set`,
  `has`, `delete`, `size`, `clear`, `keys`, `values`, `entries` and
  `map::new`; `Set`'s `add`, `has`, `delete`, `size`, `clear`, `keys`,
  `values`, `entries` and `set::new`; `WeakMap`'s `get`, `set`, `has`,
  `delete` and `weak_map::new`; and `WeakSet`'s `add`, `has`, `delete`
  and `weak_set::new`. Not `for_each`, which runs what it's given, nor a
  `WeakRef` or a `FinalizationRegistry`, which the host keeps for a
  callback.
- **It's a binding's, which runs nothing, of a JS object.** It's refused,
  saying why: on a function that isn't a binding, whose body is the
  program's; on a binding that isn't said to run nothing,
  `#[rust_js::runs_nothing]` or a property's read, since code it runs
  could keep what it's given anywhere; and on one with no collection, a
  JS object it's called on or gives.

```rust
#[cfg_attr(rust_js, rust_js::link_name = "set")]
#[cfg_attr(rust_js, rust_js::runs_nothing)]
#[cfg_attr(rust_js, rust_js::keeps_in_collection)]
pub fn set(&self, key: &K, value: V) {
```

### Why a mark of its own

- **`runs_nothing` can't say it.** 774 of the webapi crate's bindings run
  nothing, and many keep what they're given where the host reaches it, an
  event handler's setter first. Reading "keeps it in itself" into
  `runs_nothing` would make each of those wrong.
- **One mark per job.** `runs_nothing` says no code runs. This one says
  where what's given goes. A collection's method needs both, and saying
  both reads as what JS's `Map` does. A mark that implied `runs_nothing`
  would be a second way to say it runs nothing.
- **On each method, not on the type.** A collection's type has methods
  that run what they're given, `for_each`, and its maker is a function of
  a module, `map::new`, not a method. A method's own mark covers each, and
  a reader sees it beside the method's `runs_nothing`.

## Alternatives

- **Recognize `js::Map` and `js::Set` by name**: a second catalog of
  collections beside the js crate's own declarations, which a binding of
  another library couldn't join.
- **Leave collections as code rust-js can't see**: every cell kept in one
  is any cell of its type, so React DOM's tag caches check Fizz's `Batch`,
  rd201.

## Consequences

- A cell kept in a JS collection the crate makes, and gives no code
  rust-js can't see, is a cell of its own (ADR 0863, amended).
- A binding of another library's collection says the same with the same
  mark.
- The mark is trusted, as `runs_nothing` is: a program that replaces
  `Map.prototype.set` with its own is trusted to keep what it's given in
  the map.

## What holds it

- `test/lowering-cell-flow.test.ts`, "a collection's mark is refused where
  it can't be said", and the analysis's tests (ADR 0863, amended).
