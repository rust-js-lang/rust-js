# 1159. A downcast is checked, `js::downcast`

Status: Accepted (2026-11-15, #1866's follow-up, the lead's decisions).

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior is kept. TypeScript's generic assertions cast unchecked; a
Rust `&HTMLButtonElement` must be one, so the cast is tested, an
`instanceof` upstream doesn't write.

## Context

TypeScript's lib.dom gives `closest`, `querySelector` and
`querySelectorAll` three overloads each:

```ts
closest<K extends keyof HTMLElementTagNameMap>(selector: K): HTMLElementTagNameMap[K] | null;
closest<K extends keyof SVGElementTagNameMap>(selector: K): SVGElementTagNameMap[K] | null;
closest<E extends Element = Element>(selectors: string): E | null;
```

The third is an unchecked cast: the caller names any element type. The
vercel UI kernel's `composedClosest<T extends Element>` relies on it
(Codex's port, rust-js-lang/vercel-ui-sdk-in-rust). webapi's `closest`
gives an `Element`, and a Rust program had no safe way to have the
button it may be: `js::cast` is `unsafe`, and app-level `unsafe` isn't
acceptable.

## Decision

**`js::downcast::<T>(value)` is the value where it's a `T`, else `None`**:

```rust
pub fn button_of(element: &'static Element) -> Option<&'static HTMLButtonElement> {
    js::downcast::<HTMLButtonElement>(element)
}
```

```js
export function buttonOf(element) {
  return element instanceof HTMLButtonElement ? element : undefined;
}
```

- **Of a class JS can test a value by**, a binding's or a
  `#[rust_js::class]` struct's (`T: js::Classed`), its test the one
  `js::Is::when` makes (`untagged::kind_test_of`). Of a type parameter,
  whose class JS doesn't know where it's called, refused, saying what to
  write: a generic function's `T` would need its class as an argument,
  which upstream's signature doesn't have.
- **Read once**: what it's given that runs code is held first, `const
  value = pick(elements);`.
- **Its `None` is `undefined`**, never `null`, `#[rust_js::optional]` on
  the binding, so no exit gives it `?? undefined` (ADR 0030, amended).
- **It adds an `instanceof`** upstream doesn't write, false of an object
  of another realm, an iframe's element, as JS's own `instanceof` is: a
  value TypeScript's cast would have given, this gives `None`. Documented
  in the guide.

**No tag-typed `closest(Button)`** (the lead's decision, A4): a CSS type
selector matches an element's local name in any namespace, so
`closest("a")` finds an SVG `<a>`, and `closest("button")` an element of
another namespace a page made, `createElementNS(other, "button")`.
Typed `&HTMLAnchorElement`, Rust would call an anchor's methods on what
isn't one. TypeScript accepts that; Rust can only by an exception of ADR
0262's, the user's to grant (logged for Tu: tag-typed overloads of the
names unique across HTML, SVG and MathML, A1). So `closest(&str)` stays
as it is, an `Element`, and a program downcasts what it finds.

## Alternatives

- **An unchecked generic binding, `closest<E: IsA<Element>>`**: unsound,
  or an `unsafe fn` an app calls.
- **A tag overload checked after, `closest("button")` then
  `instanceof`**: wrong, not slower: where the nearest `button` isn't an
  `HTMLButtonElement`, the right one may be further up, and it gives
  `None` (A2).
- **A namespaced selector, `html|button`**: `closest` has no namespace
  prefixes without a stylesheet's `@namespace`.

## Consequences

- `composedClosest<T extends Element>`'s caller-chosen `T` is declared
  `Element | null` in the port, a recorded difference for Tu.
- `test/js-downcast.test.ts`: its JS, run against classes as a page has
  them, a button and a `div`; read once; a type parameter refused.
- Mutations downcast-of-a-type-parameter-allowed, downcast-value-read-twice,
  downcast-unchecked (`binding_calls.rs`).
- Guide: libraries/README.md, interop/bindings.md's table.
