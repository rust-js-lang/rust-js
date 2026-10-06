# 0198. An optional handler passed on to an element is that handler

Status: Accepted. Extends [0041](0041-react.md). Amended by
[0224](0224-typed-intrinsic-elements.md): a tag's handler gets its
element's event, which a handler of any element's takes `upcast`,
`f(e.upcast())`, the event itself in JS: the handler is still `f`.

## Context

A component's handler that its caller may not give, react.dev's
`Button`'s `onClick`, is an `Option<Box<dyn Fn(&event::Mouse)>>`, and an
element's handler is a closure, `impl Fn(&event::Mouse)`, whose parameter
Rust infers from it: neither an `Option`, nor a trait that takes one and a
closure, as Rust can't infer a closure's parameter from such a trait, nor
take both. So it's passed on in a closure that calls it, if it's there:

```rust
jsx! { <button onClick={move |e| if let Some(f) = &on_click { f(e) }}>{children}</button> }
```

which was that closure in JS, where a person passes the handler on:

```js
const onClick$1 = (e) => {
  if (onClick != null) {
    onClick(e);
  }
};
```

## Decision

**An event's handler that's `(e) => { if (f != null) { f(e); } }` is `f`:
`<button onClick={onClick}>`.** React ignores what a handler returns, and
no handler, `undefined`, does what one that calls nothing does.

- **An event's only**, `on` and a capital: anywhere else a function's
  identity, or what it returns, may be what's used.

## Why

- **It's the JS a person writes**, and it's exact for an event: a JSX test
  finds the handler the element is given is the one its caller gave, and
  none where none is given.
