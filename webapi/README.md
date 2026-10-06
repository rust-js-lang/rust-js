# webapi: the browser for rust-js

The `webapi` crate declares the web platform for rust-js programs: DOM
bindings **generated from W3C's WebIDL**, the same source TypeScript's
`lib.dom.d.ts` and Rust's `web-sys` come from. It holds declarations only,
so it's never compiled to JS: a program calls what it declares, and the
calls become plain JS. See [ADR 0024](../docs/decisions/0024-web-crate.md).

```rust
use webapi::events::Click;
use webapi::tags::Button;
use webapi::{document, element, event_target, mouse_event, node};

let b = document::create_element(document, Button);     // document.createElement("button"), a HtmlButtonElement
node::set_text_content(b, "+");                          // b.textContent = "+"
event_target::add_event_listener(b, Click, Box::new(move |e| {
    mouse_event::client_x(e);                            // e is the PointerEvent a button's click is
}));
element::append(app, b.into());                          // app.append(b)
```

- Each interface is a type (`Element`, `HtmlInputElement`) and a module of
  its members (`element`, `html_input_element`).
- Attributes are a getter and, if writable, a setter:
  `html_input_element::value(i)`, `html_input_element::set_value(i, "x")`.
- Inheritance is `Deref`: an `&HtmlButtonElement` goes wherever an
  `&Element` or `&Node` is expected.
- Each event's name and each tag is a type whose value is its string
  ([ADR 0223](../docs/decisions/0223-webapi-event-and-tag-maps.md)), from
  `@webref/events` and `@webref/elements`: a listener gets the event its
  target and name give it, and `create_element` the element its tag is. A
  name they don't know is `add_event_listener_named`'s, or
  `create_element_named`'s.
- `unchecked_from` is a cast:
  `html_input_element::unchecked_from(document::create_element_named(document, "input"))`.
- A result that may be `null` is an `Option`:
  `document::get_element_by_id(document, "app").expect("the page has an #app")`.
- A promise is a `Promise<T>`, the js crate's, to `.await`: `window::fetch(window, url.into()).await`
  ([ADR 0029](../docs/decisions/0029-async-await.md)). `js::settle(p).await` is a `Result`.
- Binary data is JS's `ArrayBuffer` and `Uint8Array`, the js crate's: `response::bytes(r).await`.
- A union is an untagged enum, whose value is the member itself ([ADR 0215](../docs/decisions/0215-webapi-unions.md)):
  `element::before(el, "text".into())` is `el.before("text")`, and
  `response::new_with_body(blob.into())` is `new Response(blob)`. An optional argument adds a form:
  `window::fetch_with_init(window, url.into(), init)`, `text_encoder::encode_with_input(e, "hi")`.
- An options object is a struct of `Option` fields, the rest `..Default::default()`:
  `RequestInit { method: Some("POST"), body: Some(json.into()), ..Default::default() }`.
- A constructor is `new`, `event::new("ping")`, and `request::new(url.into())`.
- A namespace is a module: `web_assembly::compile(bytes).await` is
  `await WebAssembly.compile(bytes)`. An `object` parameter takes any Rust value
  as `&dyn Any`, such as a struct for an import object.

## Use it

```bash
bun run build                                                    # its metadata, for the host
./target/debug/rust-js app.rs -- --extern webapi=target/libwebapi.rmeta --extern js=target/libjs.rmeta -L target
```

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
