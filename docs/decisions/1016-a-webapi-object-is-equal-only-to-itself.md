# 1016. A webapi object is equal only to itself

Status: Accepted (the user's decision). Extends
[0864](0864-a-type-equal-only-to-itself-keys-a-set-by-itself.md) (a
`PartialEq` of `ptr::eq` alone is `===`) to a binding crate's classes,
and [0024](0024-web-crate.md)'s webapi crate.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. Two references
to a DOM object are equal exactly when they're one object, which is
JS's `===` of two objects; `includes` finds what `===` finds of what's
never `NaN`.

## Context

React DOM's event replaying keeps the containers a blocked event may
target, and asks whether one is among them:

```js
if (targetContainer !== null && targetContainers.indexOf(targetContainer) === -1) {
```

The port's `Vec<&EventTarget>` had no `contains`: webapi's classes had no
`PartialEq`, so `target_containers.contains(&target_container)` and `a ==
b` didn't compile, and the port compared each by hand, `!targetContainers.some(
(existing) => existing === targetContainer)`.
ADR 0864 already writes `===` for a type whose `PartialEq` is `ptr::eq`
alone, but it reads that body, the crate's own or a library's contract.
webapi is a binding crate: only its metadata is built, its bodies never
read, and it has no contract.

## Decision

**Each webapi class is `PartialEq` and `Eq` by identity: its `eq` is
`ptr::eq`, natively, and the binding of JS's `===`.** The generator
writes it once for every interface:

```rust
/// `a === b`: the same object.
impl PartialEq for EventTarget {
    #[cfg_attr(rust_js, rust_js::link_name = "===")]
    fn eq(&self, other: &Self) -> bool {
        core::ptr::eq(self, other)
    }
}

impl Eq for EventTarget {}
```

```rust
targets.contains(&target)          // targets.includes(target)
a == b                             // a === b
a != b                             // of two Option<&Node>: a != b
nodes.iter().position(|&n| n == node)
```

- **A binding crate's type whose `PartialEq<Self>` has an `eq` bound to
  `===`, and std's `ne`, is one whose `==` is its address's**
  (`Recognition::eq_bound_to_strict_equality`, which
  `std_impls::eq_by_address` asks of a type of a crate with no
  contract). Everything ADR 0864 does with such a type follows: `==` is
  `===`, an `Option` of one `==` against another, a struct, tuple or
  `Vec` holding one compares it by `===`, never `$eq` (ADR 1015), and a
  generic function's `PartialEq` dictionary is `{ eq: (a, b) => a === b }`.
- **`==` of a reference to one is its**: `eq_by_address` answers through
  references, as `js_eq` does, so an `Option<&Node>`'s is `a != b`.
- **`contains` of what `===` compares and is never `NaN` is
  `includes`**: an object equal only to itself, a binding's or the
  crate's (`std_impls::contained`). `indexOf(x) >= 0` stays for a float
  and a value of any shape, whose `NaN` `includes` would find (ADR 0966).
- **A binding crate's `eq` bound to other JS, or its own `ne`, is
  refused**, as before: no binding names what `!=` would run.
- No `Hash`: a `HashSet<&Element>` is still refused by rustc. A port that
  needs one is the next decision.

## Why

- **It's the Rust an engineer writes**, `contains` and `==`, and the JS
  one writes of it, `includes`; upstream's `indexOf(..) === -1` is the
  same answer, an accepted port difference by the user's decision.
- **The binding names its JS** as every binding does, `===` the
  binding of `js::strict_equals` already; its body is what the type's
  identity is natively, as a reference to a JS object is one object.

## Alternatives

- **Read the body, `ptr::eq`, as of the crate's own type**: a binding
  crate's MIR isn't in its metadata, and a second recognizer of MIR for
  what THIR's says would be a second owner of the rule.
- **Every JS object binding equal by `===`, whatever its impl says**: a
  rule no binding states, which a binding crate's own `eq` would
  contradict.
- **`#[derive(PartialEq)]`**: of a `PhantomData`, every two equal.

## Consequences

- `test/corpus/identity_partial_eq.js`: a `contains` of the crate's own
  type equal only to itself is `includes`, not `indexOf(..) >= 0`.
- Each of webapi's 703 classes gains ten lines in `webapi/src/lib.rs`.

## What holds it

- `test/webapi-bindings.test.ts`, "a webapi object is equal only to
  itself, == its ===": `includes`, `===`, an `Option`'s `!=`, a derived
  struct's `===` and `position`, run; a binding crate's own `ne` and an
  `eq` bound to `isEqualNode` refused.
- Mutations `webapi-eq-binding-unread`, `eq-by-address-of-references-unseen`,
  `contains-of-identity-index-of`, `bound-eq-own-ne-trusted`,
  `bound-eq-form-unchecked`.
