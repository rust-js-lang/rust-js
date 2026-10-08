# 0260. A binding's callback parameter, and IntersectionObserver

Status: Accepted. Extends [0024](0024-web-crate.md) and
[0219](0219-webapi-sequences.md).
Amended by [0282](0282-event-listener-methods.md): event listener methods
take closures directly; other WebIDL callbacks retain their existing forms.

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev's TopNav shadows its bar once the page has scrolled, which an
`IntersectionObserver` tells it:

```tsx
const observer = new IntersectionObserver(
  (entries) => { entries.forEach((entry) => setIsScrolled(!entry.isIntersecting)); },
  {root: null, rootMargin: `0px 0px`, threshold: 0}
);
observer.observe(scrollDetectorRef.current!);
```

The webapi crate read no Intersection Observer spec, and skipped every
function that takes a WebIDL callback but `addEventListener`'s, so
`requestAnimationFrame` and `queueMicrotask` weren't bound either.

## Decision

- **The webapi crate reads `intersection-observer`**: `IntersectionObserver`
  and `IntersectionObserverEntry`, and the dictionary its constructor takes.
- **A callback parameter is a closure**, `Box<dyn FnMut(..) -> R>`, given
  what JS calls it with as a function's parameters are typed, `None` of
  one that may be `null`; a callback of an argument of any type is skipped.
- **An event handler attribute's callback isn't**, `onclick`'s: a listener
  is `add_event_listener`'s, typed by its event.

```rust
intersection_observer::new_with_options(
    Box::new(move |entries, _| entries.iter().for_each(|entry| seen(intersection_observer_entry::is_intersecting(entry)))),
    IntersectionObserverInit { root_margin: Some("0px 0px"), threshold: Some(0.0.into()), ..Default::default() },
)
```

```js
new IntersectionObserver((entries) => { entries.forEach((entry) => { seen(entry.isIntersecting); }); }, { rootMargin: "0px 0px", threshold: 0, .. })
```

## Why

- **They're the browser's own**, as the site uses them.
- **It's tested**: a compiler test watches an element with an observer
  whose callback it calls, and asks for an animation frame.
