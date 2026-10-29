# 1015. What holds an object `===` compares is never `$eq` of it

Status: Accepted. Amends [0053](0053-partial-eq.md) (when `==` is one
`$eq`), [0864](0864-a-type-equal-only-to-itself-keys-a-set-by-itself.md)
(a type equal only to itself) and [0612](0612-unknown-equality-is-strict.md)
(a value of any shape). Fixes #397 (rd104). Amended: a key that holds one
finds it by itself too, which amends [0121](0121-value-keys.md) (#398).

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior is preserved. It was a wrong answer.

## Context

React's `processDispatchQueueItemsInOrder`, in react-dom-bindings,
compares each listener's fiber with the one before it:

```rust
let mut previous_instance = None;
for DispatchListener { instance, .. } in dispatch_listeners.iter().rev() {
    if previous_instance != Some(instance) && (event.is_propagation_stopped.get())() {
        return;
    }
    ..
    previous_instance = Some(instance);
}
```

`instance` is a `&Option<Rc<Fiber>>`, and the reconciler's `Fiber` is
equal only to itself: its `PartialEq` is `ptr::eq`, its `Hash`
`ptr::hash` (ADR 0864). rust-js printed

```js
if (!$eq(previousInstance, $some(instance)) && event.isPropagationStopped()) {
```

`$eq` compares two objects by what's in them, key by key, and never by
`===`: it walked the fibers' `return`, `child` and `alternate`, a cycle,
until `RangeError: Maximum call stack size exceeded`. 116 tests of seven
React DOM suites failed. Where nothing is cyclic, two fibers alike were
equal, where Rust says they aren't.

ADR 0053 writes one `$eq` for a type with no custom part, `custom_eq`'s
answer. `custom_eq` took any type whose `==` is `===`, `js_eq`'s answer,
for no custom part, as a string or a number is to `$eq`. But `js_eq`
answers for objects too since ADR 0864: a type equal only to itself, and
an `Rc` of one. So an `Option` of a reference to an `Option` of an
`Rc<Fiber>` had "no custom part", and was one `$eq`. A tuple, a `Vec` and
a derived struct holding one were too, in one crate as across two: the
consumer read the library's contract, which lists `Fiber` as equal only to
itself (ADR 0864, amended), and answered as the library would. A type
whose `PartialEq` is `ptr::eq` alone, with no `Hash`, wasn't in `js_eq`:
it's custom by its impl, which was called.

## Decision

**A type whose `==` is `===` and that may be an object is a custom part
of what holds it** (`custom_eq`, of `eq_of_objects`): a value of any
shape (ADR 0612, as before), a type equal only to itself, a library's
too, an `Rc` of one, and an untagged enum with a payload of one, which is
its value (ADR 0214). So what holds one compares it part by part, as ADR
0053 does a hand-written `eq`'s, and the part itself is `===`, or `==` of
an `Option` of it:

```rust
fn same_instance(previous: Option<&Option<Rc<Fiber>>>, instance: &Option<Rc<Fiber>>) -> bool {
    previous == Some(instance)
}

#[derive(PartialEq)]
struct Listener {
    instance: Option<Rc<Fiber>>,
    name: String,
}

fn same_listener(a: &Listener, b: &Listener) -> bool {
    a == b
}
```

```js
function sameInstance(previous, instance) {
  const right = $some(instance);
  return previous == null || right == null
    ? previous == right
    : $someValue(previous) == $someValue(right);
}

function sameListener(a, b) {
  return a.instance == b.instance && a.name === b.name;
}
```

- **One answer.** `js_eq` still says how JS's comparison agrees with
  `==`; `custom_eq` asks it and, of its yes, whether the value may be an
  object, which `$eq` compares otherwise. Nothing else decides when a
  `$eq` is written.
- **Across crates as in one.** A consumer's `custom_eq` of a library's
  type asks the same `equal_only_to_itself`, its contract's answer: the
  library's contract needs nothing new. A library type whose `PartialEq`
  is `ptr::eq` without a `Hash` is custom by its listed impl, which the
  consumer compiles itself where the library doesn't export it (ADR 0914).
  (Amended by ADR 0864's last amendment: the contract lists it, and its
  `==` is `===` there too.)

## Why

- **`$eq` can't know a type.** ADR 0053 rejected `$eq` looking impls up
  at run time: JS values don't carry their types. A custom part compared
  by the compiler is how a hand-written `eq` already decides inside a
  derived one; an identity is one more.
- **Only what holds an object pays.** Strings, numbers and fieldless
  enums are compared by `$eq` as by `===`, and keep their one `$eq`.

## Alternatives

- **`$eq` returning `true` of `a === b` objects first.** Right of two
  references to one fiber, and faster, but two distinct fibers alike are
  still walked and found equal. Rejected: it hides the wrong answer.
- **Refuse `==` of what holds one.** Rust's `==` of it is ordinary and
  has a correct JS form. Rejected.

## Consequences

- The `Option<&Option<Rc<Fiber>>>` compare reads `previous == null ||
  right == null ? .. : $someValue(previous) == $someValue(right)`, where
  upstream writes `instance !== previousInstance`: a nested `Option`'s
  inner `None` is boxed (ADR 0051). Printing upstream's form needs a
  nested `Option` whose inner `None` is the field's `null` and whose
  outer `None` is `undefined`, a representation decision of its own; open.
- A `HashMap` keyed by a derived struct holding a type equal only to
  itself is a `$KeyMap` keyed by its fields: a separate wrong answer,
  #398, fixed by the amendment below.

## What holds it

- Corpus `eq_of_identities_inside`: an `Option<&Option<Rc<Fiber>>>`,
  nested `Option`s, a tuple, a `Vec`, a derived struct and an untagged
  enum of a cycle of fibers, against native Rust.
- `test/crates/pairs/library_eq`: a consumer comparing a library's
  `Fiber` (equal only to itself), `Update` (`ptr::eq` alone), derived
  `Point` and `Dispatch` (holding a fiber), and hand-written `Version`
  through `Option`s, references, `Rc`s, `Vec`s and the library's generic
  functions, against native Rust; and "a consumer's == of what holds a
  library's type equal only to itself is never $eq" holds its JS.
- Mutations `identity-inside-compared-by-contents`,
  `identity-inside-unseen`, `identity-inside-untagged-unseen` and
  `unknown-compared-by-contents` (`scripts/mutations/lower/std_impls.ts`).

## Amendment: a key that holds one finds it by itself too

Case ([0262](0262-when-rust-and-js-disagree.md)): results changed, so
Rust's behavior is preserved. It was a wrong answer (#398).

A set's or a map's key found by its value, a derived struct, a tuple, an
`Option`, a `Vec` or an enum, is a `$KeyMap` or a `$KeySet` (ADR 0121),
which keeps each entry under `$key` of the key, a string of what's in
it. A key holding a type equal only to itself was one too:

```rust
#[derive(PartialEq, Eq, Hash)]
struct Key {
    fiber: Rc<Fiber>,
    lane: u32,
}
..
m.insert(Key { fiber: Rc::clone(&a), lane: 1 }, "a");
m.insert(Key { fiber: Rc::clone(&b), lane: 1 }, "b");
```

The key's derived `Hash` and `Eq` find the fiber by its address, its
own `ptr::hash` and `ptr::eq`, so two fibers alike are two keys. `$key`
wrote the fiber's fields: `a` and `b` were one key, `len()` 1 where
Rust's is 2, and a cycle of fibers, a fiber's `alternate`, overflowed the
stack. It's the same question `==` asked of what holds one, of a map's
key.

**A key found by its value that holds a part `===` finds by itself, one
equal only to itself, a library's too, or an `Rc` of one, is kept under
`$key` of its parts, each such part its `$id`, the number that object
is known by; and the rest of it as it is.**

```js
const m = new ($keyMap((key) => ({ ...key, fiber: $id(key.fiber) })))();
m.set({ fiber: a, lane: 1 }, "a");
m.set({ fiber: b, lane: 1 }, "b");
```

- **One answer.** Which parts are found by themselves is `eq_of_objects`'s,
  `==`'s question above; `std_types/map.rs`'s `holds_identity` asks it
  through the key's parts, and `key_parts` writes the function of them:
  a struct `{ ...key, fiber: $id(key.fiber) }`, a tuple `[$id(key[0]),
  key[1]]`, a `Vec` `key.map($id)`, an `Option` its `None` or its
  `Some`'s, `$id` itself of an `Option` of one, and an enum each variant's
  that holds one. Everything that makes a map of a key, `new`, `collect`,
  `clone`, `From`, a set read from JSON and generic code's map class
  given by its caller (ADR 0466), asks `map_class`, which makes it.
- **`$id(object)`** is a number from a `WeakMap` of the objects it's
  given, the same of one object each time and another of each other, and
  `None` of `None`; `$keyMap(parts)` and `$keySet(parts)` are a
  `$KeyMap` and a `$KeySet` whose key is `$key(parts(key))`, as
  `$ordMap(cmp)` is an `$OrdMap` of its `cmp`. A key holding none is a
  `$KeyMap` as before.
- **Across crates as in one.** A consumer's key holding a library's type
  equal only to itself asks the library's contract, as `==` does, and
  the library's own map of it is made the same: the consumer reads it and
  adds to it.
- **What can't be found by itself is refused**, never keyed by what's in
  it: a key's untagged enum with a payload of one, whose number `$id`
  could be another payload's; an `Option` whose `Some` is boxed (ADR
  0051); and a `Cow` of a slice of them, which `$cowKey` finds by what it
  borrows, "values of type `Cow<'_, [&Fiber]>`".
- **A B-tree finds its keys by their `Ord`**, not their `Hash`: one whose
  `Ord` is its own is an `$OrdMap` of its `cmp` (ADR 0404), and one whose
  `Ord` is derived all through, a fiber's too, which std's `Ord` contract
  requires to agree with its `Eq`, is kept by its parts as above.

### Why

- **It's what the key's own impls say**, one fiber, one key, as a JS
  `Map` keys an object.
- **Only a key that holds one pays**: a function for its parts and a
  `WeakMap` lookup a part. Every other key is what it was.

### Alternatives

- **A hash table in the runtime by the key's `Hash`**, as ADR 0121
  considered: `ptr::hash` writes an address (ADR 0562), whose ranges a
  `$KeyMap` of many fibers would use up, and every key would pay for it.
- **Refuse such a key**: Rust's map of it is ordinary, React's keyed by
  a fiber and a lane, and has a correct JS form. Rejected.
- **`$key` telling an identity apart by itself**: a fiber is a plain
  object as a struct of values is, so nothing at run time says which.

### What holds it

- Corpus `identity_inside_keys`: a `HashMap` and a `HashSet` of a derived
  struct holding an `Rc<Fiber>`, `entry`, `remove`, `contains_key` and
  `clone`, a tuple key, an `Option` of a fiber and of the struct, a
  `Vec` of fibers, a struct of the struct, an enum's tuple and struct
  variants, and generic code given the set's class, of a cycle of
  fibers, against native Rust.
- `test/crates/pairs/identity_inside_keys`: a consumer's map of a
  library's key and its own tuple set, and the library's map of its key
  read and added to, against native Rust; "a key holding a library's type
  equal only to itself finds it by itself" holds both crates' JS.
- `test/lowering-collections.test.ts`, "a type equal only to itself keys
  a JS Set by itself": a `Cow` of them refused.
- Mutations `identity-in-key-by-contents`,
  `identity-in-key-library-by-contents`, `identity-in-key-fields-unseen`,
  `identity-in-key-option-by-contents`, `identity-in-key-option-none-read`,
  `identity-in-key-variant-skipped`, `identity-in-key-items-unmapped`,
  `identity-in-key-tuple-part-kept`, `identity-in-key-nested-struct-kept`
  and `identity-in-cow-key-by-contents` (`lower/std_types/map.ts`);
  `id-of-each-object-one` and `id-of-one-object-new` (`runtime/id.ts`),
  `key-map-parts-unread` (`runtime/key_map.ts`) and
  `key-set-parts-unread` (`runtime/key_set.ts`).
