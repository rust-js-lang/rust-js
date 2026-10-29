# 1024. A mixin its interfaces share is a type, the union of them

Status: Accepted (rd193, a port gap; P1). Extends
[0024](0024-web-crate.md)'s webapi generator and applies
[0229](0229-union-parameters.md)'s `of` to a union of classes.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. A value of it is
the document or the shadow root it is, each member read as JS reads it.

## Context

React DOM's Float keeps its hoistables in a `HoistableRoot`, Flow's and
TypeScript's `Document | ShadowRoot`, and asks it what both have:
`root.querySelector(..)`, `root.querySelectorAll(..)`, `root.nodeType`,
`root.ownerDocument`, and keys a `Map` by it. webapi had no type of
either: the port typed it as a `Node` and cast it to a `Document` at each
query, `webapi::document::unchecked_from(root).query_selector(..)`,
which printed upstream's JS but claimed a shadow root is a document. A
union enum of the two, `match root { Document(d) => d.query_selector(s),
ShadowRoot(r) => r.query_selector(s) }`, would print a test of which it
is before each call, where upstream calls the method.

## Decision

**A WebIDL mixin its interfaces share is a type of its own, the union of
them**, generated as an interface is (`UNIONS` in webapi/generate.ts):

- `DocumentOrShadowRoot`, `#[rust_js::types = "Document | ShadowRoot"]`,
  is a `Node`, the nearest ancestor both have, by `Deref`;
- its members are each mixin each of them includes, itself or by an
  ancestor: `ParentNode`'s (`query_selector`, `query_selector_all`,
  `children`, `append`, ..), `NonElementParentNode`'s
  (`get_element_by_id`) and `DocumentOrShadowRoot`'s own
  (`active_element`, `style_sheets`, `adopted_style_sheets`, ..), typed as
  the first interface's TypeScript lists them, `query_selector_all`'s a
  `NodeList<Element>`;
- each of them is one, and what extends one, `IsA<DocumentOrShadowRoot>`
  of `Document`, `XMLDocument` and `ShadowRoot`, and
  `DocumentOrShadowRoot::of(document)` is the value itself, as a union
  enum's `of` gives the member a parameter was given (ADR 0229); a node
  that's one is `document_or_shadow_root::unchecked_from(node)`, as of
  every interface.

```rust
pub fn hoisted(root: &webapi::DocumentOrShadowRoot) -> (f64, bool) {
    let links = root.query_selector_all("link").length();
    (links, root.node_type() == node::DOCUMENT_NODE)
}

pub fn hoisted_of_document() -> (f64, bool) {
    hoisted(webapi::DocumentOrShadowRoot::of(document))
}
```

```js
export function hoisted(root) {
  const links = root.querySelectorAll("link").length;
  return [links, root.nodeType === 9];
}

export function hoistedOfDocument() {
  return hoisted(document);
}
```

Its declaration is `hoisted(root: Document | ShadowRoot)`.

## Why

TypeScript's union of classes has the members they share, called on the
value as it is; so does this type, without a test of which it is. The
mixin is WebIDL's name for what the two share, so the type has it.
`of` is a function, not `AsRef` or `From`: a call through std's
forwarding impls, `document.as_ref()` of a `&Document`, needs a
dictionary of the binding's impl, which rust-js doesn't make.

## Alternatives

- **A union enum, `Document(&Document) | ShadowRoot(&ShadowRoot)`**: what
  a union a JS API gives is (ADR 0215), but each call is a `match`, and
  its JS a test of which it is.
- **A `ParentNode` trait both implement**, `&dyn ParentNode`: a trait
  object of a binding's methods, and one trait per mixin where the root
  needs three and `Node`'s.
- **Every mixin a type**: none other has a port that needs it; `UNIONS`
  is where one is added.

## Consequences

- One mixin, `DocumentOrShadowRoot`, is a type; a mixin's own TypeScript
  interface, `DocumentOrShadowRoot` of lib.dom, isn't what it declares
  as: that has only the mixin's members.
- `instanceof DocumentOrShadowRoot` names no class; nothing makes one.

Held by `test/compiler.test.ts`, "the webapi crate's bindings become
plain JS" (test/web_forms.rs's `hoisted`, its JS and a run), and
`test/webapi-coverage.test.ts`, "the webapi crate is what its generator
writes". No mutation: the change is the generator's (ADR 1021).
