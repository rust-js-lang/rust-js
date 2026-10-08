# 0224. A tag's element reaches its handlers and its `ref`

Status: Accepted. Extends [0041](0041-react.md), [0075](0075-jsx-only-elements.md)
and [0223](0223-webapi-event-and-tag-maps.md).

Case: N, A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

In TypeScript, `<button>` is a `HTMLButtonElement` all the way up:
@types/react's `IntrinsicElements.button` gives it
`ButtonHTMLAttributes<HTMLButtonElement>`, so its `onClick` handler's
`e.currentTarget` is `EventTarget & HTMLButtonElement`, and its `ref` takes
a `Ref<HTMLButtonElement>`. Laminar does the same with
`button: HtmlTag[dom.HTMLButtonElement]`, whose element's `ref` is a
`dom.html.Button` ([research](../research/type-foundations.md#layer-3-react)).

In rust-js every tag was the same `react::Element`, so the element type was
lost:

```rust
<button onClick={|e| { e.current_target(); }}>   // &webapi::Element, not the button
<button ref={input_ref}>                          // any Ref<Option<H>>, H free
```

What was missing was only a type: the JS of a `<button>` is already the
button. And TypeScript types a tag only in its props: what JSX makes is a
`JSX.Element` whatever its tag, so `cond ? <span/> : <b/>` is one type.

## Decision

**`react::Element` is of a DOM element, `Element<T = webapi::Element>`:
while `jsx!` builds a tag's, it's that tag's, `Element<webapi::HtmlButtonElement>`,
and gives it to its handlers' events and its `ref`; what it makes is an
`Element`. The JS doesn't change: the element is a type parameter, and
every conversion is the value itself.**

```rust
let input = use_ref(None::<&'static webapi::HtmlInputElement>);
let label = if busy { jsx! { <span>{"…"}</span> } } else { jsx! { <b>{"Save"}</b> } };
jsx! {
    <form>
        <input ref={input} />
        <button onClick={|e| webapi::html_button_element::set_disabled(e.current_target(), true)}>{label}</button>
    </form>
}
```

```js
const label = busy ? <span>…</span> : <b>Save</b>;
// ..
<input ref={input} />
<button onClick={(e) => { e.currentTarget.disabled = true; }}>{label}</button>
```

- **Each tag's function is of its element**, `html::button() -> Element<webapi::HtmlButtonElement>`,
  as `webapi`'s `Tag` gives it (ADR 0223): 113 of React's 172 tags, 82 of
  them `HtmlElement` where `webapi` doesn't bind theirs yet. An SVG one, or
  any `webapi` has no tag of, is an `Element`, as a tag value (ADR 0220) and
  a built-in, `Fragment` say, are.
- **One `Element`, generic, not a type per tag**: a tag's, a tag value's
  and a built-in's props are the same methods, on `impl<T> Element<T>`, and
  `Element` alone still means what it did.
- **`jsx!` ends a tag's with `react::element(..)`**, a binding whose JS is
  the value itself, so what JSX makes is an `Element`: a component's
  result, a child, a branch of a conditional, an item of a list.
- **An event is of its element**, `event::Mouse<T = webapi::Element>`, as
  React's `MouseEvent<T = Element>` is: a handler is
  `on_click(self, handler: impl Fn(&event::Mouse<T>) + 'static)`, its
  closure's parameter still inferred; `current_target()` is the `&T`; and
  `target()` is an `&webapi::EventTarget`, what the DOM and @types/react
  say it is, where it was an `Element`.
- **A closure written outside the JSX names its event with the element left
  to the tag**, `move |e: &event::Change<_>| ..`, where `&event::Change`
  is any element's.
- **A handler of any element's event is widened, explicitly**:
  `<button onClick={event::Mouse::widen(on_click)}>` of a component's
  `on_click: Box<dyn Fn(&event::Mouse)>`, and an event given to one is
  `e.upcast()`, so ADR 0198's optional handler is
  `move |e| if let Some(f) = &on_click { f(e.upcast()) }`, still `onClick={onClick}`
  in JS. Each is the value itself. It's sound: a handler that takes any
  element's event takes a button's. **An associated function whose first
  parameter is `this` is a method**, as a free one is, which `widen` is.
- **An async handler is the async function itself**: `event::Mouse::spawn(async
  move |_| { .. })` is `async () => { .. }`, where a closure that calls
  `js::spawn` of an `async` block is `() => { (async () => { .. })(); }`.
  Each event runs it without waiting for it, as `spawn` would, and React
  ignores the promise, as it does what any handler returns. It's of event
  types only, so an effect, whose promise React would take for a cleanup,
  isn't given one. An `async` closure's `_` is left out, as a closure's is.
  (Amended.)
- **A `ref` holds its tag's element or one it extends**: `<button ref={r}>`
  takes a `Ref<Option<&'static U>>`, or a callback of one, where
  `HtmlButtonElement: IsA<U>` (ADR 0223). An `<input>`'s ref on a
  `<button>` is rustc's error.
- **Its `.d.ts` is a `ReactNode`**, whatever its element:
  `#[rust_js::types = "react#ReactNode<>"]`, and arguments written in
  `types` are all of TypeScript's type's, `<>` none (ADR 0196).
- **Attributes are restricted by `T`, as `ButtonHTMLAttributes<T>` does**:
  a tag takes the attributes @types/react gives it (ADR 0228). (Amended:
  they were one set for every tag.)

## Why

- **It's TypeScript's and Laminar's typing, at no cost in the JS**: the
  react.dev port's generated JS didn't change, and its 823 pages are the
  same.
- **It's checked where TypeScript isn't**: a handler of the base element is
  widened at the call, where @types/react makes every handler bivariant
  (`bivarianceHack`), which accepts the unsound direction too. And
  react.dev's table of contents listens for `resize` on the document, which
  only a window gets: by name now, as ADR 0223's events refuse it.
- **Inference stays**: one exact `Fn(&event::Mouse<T>)` bound per handler.
  Two impls, exact and widened, under a marker type, would accept both
  without `widen`, but a closure's parameter could no longer be inferred.
- **It's tested**: a JSX test's `<button>` handler sets the button's
  `disabled` through `current_target()`, an `<input>` takes its ref, a
  `<span>` and a `<b>` are the branches of one conditional, and a base
  handler is widened, to `onClick={onClick}`; an `<input>`'s ref on a
  `<button>`, a handler of an `<input>`'s event on it, and a base handler
  not widened, are each rustc's error, said as such.

## Not yet

- A `Change`'s `value` reads `target.value` still, as before.
- `native_event()` is a `webapi::Event`, where React's `MouseEvent` has the
  DOM's `MouseEvent`.

## Costs

- A breaking change to the `react` crate: a closure that names its event
  type needs `<_>` (12 in this repository), a handler of any element's event
  stored in props needs `widen`, or `upcast` given an event (2 in the
  react.dev port), and an event's `target` is an `EventTarget`.
- A tag's precision is `webapi`'s: of the HTML element interfaces, it binds
  24 so far.
