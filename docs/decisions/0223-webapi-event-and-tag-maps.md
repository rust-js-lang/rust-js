# 0223. The webapi crate knows each event's type and each tag's element

Status: Accepted. Extends [0024](0024-web-crate.md), [0102](0102-js-and-webapi.md)
and [0013](0013-fieldless-enums.md).

## Context

`webapi` took any event name and gave every listener the base `Event`:

```rust
event_target::add_event_listener(b, "click", Box::new(|e| {
    mouse_event::client_x(mouse_event::unchecked_from(e));
}));
```

so a click handler narrowed by hand, unchecked, and a misspelled `"clik"`
compiled. `create_element(document, "button")` was an `Element`, cast by
hand too.

TypeScript's `lib.dom.d.ts` ties both: `HTMLElementEventMap` gives
`"click"` its event, and `addEventListener<K extends keyof HTMLElementEventMap>`
gives the listener `HTMLElementEventMap[K]`; `HTMLElementTagNameMap` gives
`createElement("button")` a `HTMLButtonElement`. WebIDL, which `webapi` is
generated from, has neither. TypeScript's generator reads them from two
more webref packages, `@webref/events` (each event's interface, its targets
and where it bubbles) and `@webref/elements` (each element's interface)
([research](../research/type-foundations.md#layer-2-the-dom)). Scala.js's
and ReScript's DOM bindings tie neither.

## Decision

**`webapi/generate.ts` reads `@webref/events` and `@webref/elements` too,
pinned as `@webref/idl` is. Each event's name and each tag is a type whose
value is its string; a listener gets the event its target and name give
it, and an element made from a tag is that tag's interface.**

```rust
use webapi::events::Click;
use webapi::tags::Button;

let button = document::create_element(document, Button);   // &HtmlButtonElement
event_target::add_event_listener(button, Click, Box::new(|e| {
    mouse_event::client_x(e);                                 // e: &PointerEvent
}));
```

```js
const button = document.createElement("button");
button.addEventListener("click", (e) => {
  e.clientX;
});
```

- **A name is a unit struct named `#[rust_js::name]`**, `webapi::events::Click`
  and `webapi::tags::Button`, **and a unit struct so named is its string in
  JS**, `"click"`, as a fieldless variant is its name (ADR 0013). Without a
  name, a unit struct is still `undefined`. A struct, not a variant, as each
  name needs impls of its own.
- **What an event is depends on its target**, as TypeScript's maps do:
  `impl Listen<events::Click> for HtmlButtonElement { type Event = PointerEvent; }`,
  for each interface `webapi` binds that's a target of the event, or on its
  bubbling path, or extends one; the nearest's, where several are. 137 names,
  2,937 impls on 35 targets.
- **`add_event_listener`, `remove_event_listener`, and their `_with_options`
  forms, take the name's type**: generic, so Rust functions whose JS is
  `addEventListener`, beside the extern ones:
  `fn add_event_listener<T: Listen<E>, E>(this: &T, event: E, listener: Box<dyn FnMut(&<T as Listen<E>>::Event)>)`.
- **A name the data doesn't know is said so**: `add_event_listener_named(this, "my-event", ..)`,
  the string form, renamed, given the base `Event`. TypeScript's every
  `addEventListener` falls back to any string silently; here it's a choice
  at the call.
- **A tag gives its element**: `create_element(document, Button)` is a
  `&'static HtmlButtonElement`, for the 121 HTML tags; a string is
  `create_element_named`'s, an `Element`.
- **An SVG tag gives its SVG element**, as TypeScript's
  `SVGElementTagNameMap`: `create_element_ns(document, namespaces::Svg,
  svg_tags::Circle)` is `document.createElementNS("http://www.w3.org/2000/svg",
  "circle")`, a `&'static SVGCircleElement`, for SVG's 63 tags, by `SVGTag`.
  Not `Tag`'s: `createElement("circle")` makes an `HTMLUnknownElement`. A
  name SVG and HTML share, `a` or `script`, is in each, its own element. A
  string is `create_element_ns_named`'s, an `Element`. (Amended.)
- **What `webapi` doesn't bind is the nearest it does**: a click is a
  `PointerEvent`, bound for it, as a form's submit is a `SubmitEvent`; a
  `hashchange`, whose `HashChangeEvent` isn't bound, is an `Event`, and a
  `<video>`, whose `HTMLVideoElement` isn't, an `HtmlElement`. Where two
  specs give one target's name different events, twice, it's an `Event`.
- **Each interface knows its ancestors**: `unsafe impl IsA<HtmlElement> for HtmlButtonElement`,
  and for itself. `Deref` upcasts a value; `IsA` lets a bound say "this or
  what extends it", as ADR 0224 will.

## Why

- **It's TypeScript's DOM typing, from the same data**, regenerated with
  each pinned version, where scala-js-dom is written by hand and ReScript's
  webapi was generated once.
- **Without TypeScript's loophole**: no silent string fallback; an unknown
  name is a named function.
- **The JS is what a person writes**, `addEventListener("click", ..)` and
  `createElement("button")`: a name's type is its string, and the examples'
  JS didn't change when they moved to it.
- **It's tested**: a compiler test makes a button by its tag, sets its
  `disabled`, listens for a click and reads `clientX` without a cast, and a
  document's keydown reads `key`; a mouse event's reader of a keydown and a
  button's `Message` listener are rustc's errors, said as such. A named
  unit struct is its string, returned, through a generic function, and as a
  constant.
- **It's cheap**: `webapi`'s metadata builds in 0.39 s, from 0.32 s.

## Not yet

- **A checked downcast**, `instanceof` and the value: `webapi` is
  declarations only, used from its metadata (ADR 0024), so it needs a
  link-name form, `x instanceof C ? x : undefined`, or a home with a body.
- **The interfaces it falls back from**: `HashChangeEvent`,
  `HTMLVideoElement` and the rest, as `webapi` binds more.

## Costs

- `webapi/src/lib.rs` grows by 4,600 lines, an impl per interface and event.
- Two more pinned inputs, `@webref/events` and `@webref/elements`.
- The string forms are `_named`: breaking, for code written against them.
