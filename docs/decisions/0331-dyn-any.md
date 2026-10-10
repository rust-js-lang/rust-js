# 0331. A `TypeId` is its type's name, and a `dyn Any` a pair

Status: Accepted. Extends [0141](0141-std-trait-objects.md) and
[0145](0145-type-facts.md); counted by
[0314](0314-std-data-structures.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)), but D for what JS
can't keep: a `TypeId`'s hash, a closure's type, and an order of types.

## Context

A `dyn Any` was an error (ADR 0141), and so was `TypeId`, whose std holds
a pointer. `Box<dyn Any>`, a downcast, a map of `TypeId`s and a trait
whose supertrait is `Any` are how Rust keeps values of types it decides
at run time: an event's payload, a plugin's state, a type map.

## Decision

**A `TypeId` is its type's name, rustc's own `type_name`, the same in each
crate rust-js compiles; a `dyn Any` is a pair, as a `dyn Display` is
(ADR 0141), of its value and `Any`'s dictionary, `{ type_id: () => "i32"
}`.**

```rust
let items: Vec<Box<dyn Any>> = vec![Box::new(5i32), Box::new(Point { x: 1 })];
if let Some(n) = items[0].downcast_ref::<i32>() { .. }
```

```js
const items = [
  { value: 5, impl: { type_id: () => "i32" } },
  { value: { x: 1 }, impl: { type_id: () => "lib::Point" } },
];
const n = items[0].impl.type_id() === "i32" ? items[0].value : undefined;
```

| Rust | JS |
|---|---|
| `TypeId::of::<T>()`, `x.type_id()` | `"i32"`; a `dyn`'s `d.impl.type_id()` |
| `is::<T>()`, `downcast_ref::<T>()` | `d.impl.type_id() === "T"`, and `? d.value : undefined` |
| `downcast_mut::<T>()` | the pair for a number or text, the box its `value` is (ADR 0074), or its object |
| `Box<dyn Any>`'s, `Rc<dyn Any>`'s `downcast::<T>()` | `Ok` of its value, or `Err` of itself |
| `==`, a map's key | `===`, a string key |
| a `T: Any`'s, a `T: 'static`'s | its dictionary, `TAny.type_id()`, or its caller's `TId`, a type fact (ADR 0145) |

- **A trait under `Any`**, `trait Component: Any`, has `Any`'s dictionary
  as a supertrait's, so `&dyn Component` upcast to `&dyn Any` downcasts,
  and `(*c).type_id()` is its value's.
- **A binding's `&dyn Any`, any JS value, is given the value**, not the
  pair: webapi binds WebIDL's `object` so.
- **`{:?}` of a `dyn Any`** is `Any { .. }`, as std's, and `$debugAny(made())`
  of one made where it's shown, which runs what makes it.
- **Refused, loud**: a closure's `TypeId`, as closures share a name; `{:?}`
  of a `TypeId`, its hash; `<` of `TypeId`s and a `BTreeSet` of them, by
  their hashes' order; a counted `Rc<dyn Any>`'s downcast. A type whose name doesn't tell
  it apart has its hash beside its name: a `dyn` or a function pointer,
  whose higher-ranked lifetimes a name leaves out, `dyn for<'a> AsStr<'a,
  'a>` and `dyn for<'a> AsStr<'a, 'static>`, and a type of a crate linked
  at two versions. (Amended: rustc's `any-lifetime-escape-higher-rank.rs`
  found the `dyn`'s.)

## Why

- **It's the same program**: a downcast finds what Rust finds, and a
  `TypeId` is equal where Rust's is.
- **It's the JS a person writes**: a type's name, compared.
- **It's tested**: the `dyn_any` and `dyn_any_generic` corpus cases run
  downcasts of numbers, strings, structs and `Option`s, `downcast_mut`,
  `Box`'s and `Rc`'s `downcast`, a failed one given back, `TypeId`s
  compared and as map keys, a trait under `Any`, `T: Any` and `T:
  'static` in generic code and a `dyn Any + Send`, against native Rust;
  compile-fail cases refuse a closure's `TypeId`, its `{:?}` and a counted
  `Rc`'s downcast. Mutations copy `downcast_mut`'s number, leave a `None`
  unboxed, give the value back, skip the closure check, give a binding the
  pair, ask a dictionary for `5i64.type_id()`, and compare deeply.
- `docs/std-coverage.txt`: `Box` 5 of 17, `Rc` and `Arc` 12 of 23.
