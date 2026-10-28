# 1011. A binding's index signature is std's `Index`

Status: Accepted. React port gap rd68 (the coordinator, by the rules: a
missing binding is a rust-js gap).

## Context

React DOM reads collections by index: ReactDOMSelect's `options[i].value`
and `options[i].selected = true` of an `HTMLOptionsCollection`, and
ReactDOMInput's `group[i]` of a `NodeList`. TypeScript types each by an
index signature, `[index: number]: HTMLOptionElement`. The webapi crate
had only WebIDL's `item(i)`, an `Option<&Element>`, so the port wrote
`html_option_element::unchecked_from(options.item(i).unwrap())`, printed
`$unwrap(options.item(i))` and a cast upstream doesn't have.

## Decision

**TypeScript's index signature is std's `Index<usize>`, its method the
binding of `get []`, the form that owns `x[key]` reads**: `options[i]` is
`options[i]`, of the class TypeScript names, its own or its parent's of
the type it's given (`HTMLOptionsCollection extends
HTMLCollectionOf<HTMLOptionElement>`), a `NodeList<T>`'s its `T`. The
generator reads each class's signature (`coverage.ts`'s `indexed`) and
writes the impl of each whose items are a class it binds, 30 of them;
a union's, `HTMLSelectElement`'s, none.

A call of a std trait's method that resolves to a binding's impl of it,
`Index::index` here, is that binding's call (`dispatch.rs`), as a call of
the method itself is: the one way a binding is called.

Case ([0262](0262-when-rust-and-js-disagree.md)): a binding is trusted as
declared (semantics.md). As TypeScript's type, an index past the end is
JS's `undefined` taken for an item, and what's called on it throws JS's
`TypeError` where Rust's `Index` panics; `item(i)`, an `Option`, is the
read that asks.

## Consequences

Held by `test/webapi-bindings.test.ts`, "a collection's index signature
is Index, options[i]", run in Node, and the mutation
`index-of-a-binding-impl-refused` (`lower/mir/dispatch.ts`). A loop to a
collection's `length()`, an `f64` (ADR 0994), is still `0..options.length()
as usize`, `$f64ToInt(options.length, ..)`.
