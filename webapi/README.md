# webapi: the browser for rust-js

The `webapi` crate declares the web platform for rust-js programs: DOM
bindings **generated from W3C's WebIDL**, the same source TypeScript's
`lib.dom.d.ts` and Rust's `web-sys` come from. It holds declarations only,
so it's never compiled to JS: a program calls what it declares, and the
calls become plain JS. See [ADR 0024](../docs/decisions/0024-web-crate.md).

```rust
use webapi::events::Click;
use webapi::tags::Button;
use webapi::{EventTargetExt, document};

let b = document.create_element(Button); // document.createElement("button"), a HTMLButtonElement
b.set_text_content("+");                 // b.textContent = "+"
b.add_event_listener(Click, |e| {
    e.client_x();                         // e is the PointerEvent a button's click is
}, None);
app.append((b, "!"));                     // app.append(b, "!"), a rest parameter's tuple
```

- Each interface is a type (`Element`, `HTMLInputElement`), its members its
  methods (`el.append(..)`), and its constructors, statics, constants and
  casts a module of its name (`html_input_element::unchecked_from`).
  `EventTargetExt` gives every target its listener methods
  ([ADR 0282](../docs/decisions/0282-event-listener-methods.md)).
- Attributes are a getter and, if writable, a setter:
  `i.value()`, `i.set_value("x")`.
- Inheritance is `Deref`: an `&HTMLButtonElement` has `Element`'s and `Node`'s
  methods, and goes wherever an `&Element` or `&Node` is expected.
- Each event's name and each tag is a type whose value is its string
  ([ADR 0223](../docs/decisions/0223-webapi-event-and-tag-maps.md)), from
  `@webref/events` and `@webref/elements`: a listener gets the event its
  target and name give it, and `create_element` the element its tag is. A
  name they don't know is `add_event_listener_named`'s, or
  `create_element_named`'s.
- `unchecked_from` is a cast:
  `html_input_element::unchecked_from(document.create_element_named("input", None))`.
- A result that may be `null` is an `Option`:
  `document.get_element_by_id("app").expect("the page has an #app")`.
- A promise is a `Promise<T>`, the js crate's, to `.await`: `window.fetch(url, None).await`
  ([ADR 0029](../docs/decisions/0029-async-await.md)). `js::settle(p).await` is a `Result`.
- Binary data is JS's `ArrayBuffer` and `Uint8Array`, the js crate's: `r.bytes().await`.
- A union parameter takes each member as it is, `impl IntoRequestInfo`, and a union's value is
  an untagged enum, the member itself ([ADR 0215](../docs/decisions/0215-webapi-unions.md),
  [ADR 0229](../docs/decisions/0229-union-parameters.md)): `window.fetch(request, None)` is JS's `window.fetch(request)`, and
  `response::new(Some(&blob), None)` is `new Response(blob)`. An optional argument is an
  `Option`, a union's borrowed, `Some(&value)`, and a trailing `None` is no argument
  ([ADR 0443](../docs/decisions/0443-a-value-a-caller-may-leave-out-is-an-option.md)):
  `window.fetch(url, Some(init))`, `e.encode(Some("hi"))`, `window.fetch(url, None)` is `window.fetch(url)`.
- A rest parameter, `...nodes`, is a tuple, each argument of its own type, which the binding
  spreads ([ADR 0640](../docs/decisions/0640-a-rest-parameter-is-a-tuple.md)):
  `el.append((node, "text"))` is `el.append(node, "text")`, `console::log((x,))` is
  `console.log(x)`, and `console::log(())` is `console.log()`; a slice or a `Vec` of them is
  spread, `el.append(&children)` is `el.append(...children)`.
- An options object is a struct of `Option` fields, the rest `..Default::default()`:
  `RequestInit { method: Some("POST"), body: Some(json.into()), ..Default::default() }`.
- A constructor is `new`, `event::new("ping", None)`, and `request::new(url, None)`.
- A namespace is a module: `web_assembly::compile(bytes).await` is
  `await WebAssembly.compile(bytes)`. An `object` parameter takes any Rust value
  as `&dyn Any`, such as a struct for an import object.

## Use it

An app names it in its `Cargo.toml`, `webapi = { package =
"rust-js-webapi", .. }`, and `rust-js build` builds it. `bun run build`
builds its metadata for this repository's tests.

The playground compiles every program with the `webapi` crate available.

## Regenerate it

```bash
bun run generate
```

`generate.ts` holds every rule: which specs and interfaces, how WebIDL
types map to Rust, and the names. A member is generated only if rust-js
supports all its types; the run prints what it skipped, and why. To update
the platform, bump `@webref/idl` in `package.json` and regenerate. The diff
of `src/lib.rs` is what changed.
