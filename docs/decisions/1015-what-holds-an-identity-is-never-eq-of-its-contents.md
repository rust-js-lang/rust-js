# 1015. What holds an object `===` compares is never `$eq` of it

Status: Accepted. Amends [0053](0053-partial-eq.md) (when `==` is one
`$eq`), [0864](0864-a-type-equal-only-to-itself-keys-a-set-by-itself.md)
(a type equal only to itself) and [0612](0612-unknown-equality-is-strict.md)
(a value of any shape). Fixes #397 (rd104).

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
  #398.

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
