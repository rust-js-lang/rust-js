# 0215. The webapi crate's unions are untagged enums

Status: Accepted. Amends [0024](0024-web-crate.md) and
[0102](0102-js-and-webapi.md): a union is one function's parameter, of an
untagged enum (ADR 0214), where it was a function per member. Amended by
[0229](0229-union-parameters.md): the parameter takes each member as it is,
`impl IntoNodeOrStr`, where it took the enum, `.into()` at each call.

## Context

WebIDL's unions are everywhere a web API takes "one of these":
`fetch(RequestInfo)`, a `Request` or a URL; `new Response(BodyInit)`, of
six; `element.before(Node or DOMString)`. The webapi crate gave each member
a function of its own, `before` and `before_with_str`, and a constructor a
family, `request::from_url` and `request::from_request`, with a table of
the names a type couldn't give (`PRIMARY`, `RENAMES`). Two union
parameters would be a function for each pair, so only the first varied.
And `Blob`, the body of a download or an upload, wasn't bound at all.

## Decision

**A union parameter is its untagged enum (ADR 0214), whose value is the
member itself:**

```rust
element::before(el, "text".into());           // el.before("text")
element::before(el, node.into());             // el.before(node)
response::new_with_body(blob.into());         // new Response(blob)
window::fetch(window, url.into());            // window.fetch(url)
```

- **One function, the union's name its enum's:** a typedef's, `BodyInit`,
  `RequestInfo`, or its members', `NodeOrStr`. Its variants are its
  members Rust takes, of each kind JS tells apart the first: a
  `DOMString` and a `USVString` are both `Str(&'a str)`, a `long` and a
  `double` one `Number`.
- **Each member converts into it**, `From`, and so does each interface
  that extends a class member but no other: an `&HtmlElement` into
  `NodeOrStr`'s `Node`, a `File` into `BodyInit`'s `Blob`. `.into()` is
  the value itself.
- **A setter's union, and a dictionary's field's, are the enum too**:
  `RequestInit { body: Some(blob.into()), .. }`.
- **Each interface type names its JS class**, `#[rust_js::name =
  "HTMLElement"]` of `HtmlElement`, where its Rust name isn't it, for
  `instanceof`.
- **Constructors are `new` and its optional arguments' forms**, with no
  family of sources, and no table of names: `request::new(url.into())`.
- **The File API's `Blob` and `File`** are bound.

A union the crate returns is still left out: none is the whole of a
result's members yet.

## Why

- **The JS a call makes is TypeScript's**: a member passed as it is, which
  the enum's value already is.
- **One function per WebIDL function**, as TypeScript and MDN have it,
  where each union multiplied them, or wasn't given at all past the first.

## Consequences

- `window::fetch(window, url.into())`, where it was `fetch(window, url)`:
  a conversion at every call of a union, the price of one function.
- `PRIMARY` and `RENAMES` are gone from the generator.
