# 1013. A value the DOM makes text of takes any value

Status: Accepted. The user's decision (2026-10-28), React port gap d13a; amended (rd159).

## Context

React DOM gives the DOM values it doesn't make text of first:
`node.setAttribute(attributeName, value)` of a number or a `bool`,
`style.setProperty(styleName, value)`. TypeScript types each `value`
`string`, and the webapi crate took `&str`, so the port wrote `unsafe {
js::cast::<&str>(value) }` at each, a cast upstream doesn't have. The
DOM makes text of whatever it's given, by WebIDL's `ToString`
(`DOMString`), as JS's `String(value)` does.

## Decision

**A value the DOM makes text of takes any value, `impl IntoUnknown`, as
JS takes one**: the webapi generator's `STRINGIFIED` list,
`Element.setAttribute`, `Element.setAttributeNS`,
`ProcessingInstruction.setAttribute`, `CSSStyleDeclaration.setProperty`
and the `cssFloat` setter, each its `value`. `node.set_attribute("tabindex",
3)` is `node.setAttribute("tabindex", 3)`, and text, `&str` or `&String`,
is given as before. A value is given as JS has it, so what the DOM makes
of it is JS's: `true` is `"true"`, `None` `"undefined"`.

Case ([0262](0262-when-rust-and-js-disagree.md)): None: the DOM makes the
same text of the value as of `String(value)`.

## Consequences

A CSS property's own setter, `style.set_background_color(..)`, still
takes text; widening it is the list's to say. Held by
`test/webapi-bindings.test.ts`, "a value the DOM makes text of takes any
value", run in Node.

## Amendment (2026-10-29, the user's rule applied): an input's `type` and `name`

React DOM's ReactDOMInput sets `node.type = type` and `node.name = name`
of the values its props give, any of them (gap d15a), so
`HTMLInputElement`'s `set_type` and `set_name` take any value, `impl
IntoUnknown`, as this decision's list does: the DOM makes text of it by
its `ToString`, `toString` first. Converting it to text in Rust first,
`'' + value`, would ask `valueOf` first, a different text. Held by
`test/webapi-bindings.test.ts`'s "a value the DOM makes text of takes any
value" (`input.type = kind` of an object, run).

## Amendment: a select's `size` and an image's `src` (d13a)

React's ReactDOMComponent sets `select.size` and `img.src` of a prop's
value as it is, which the DOM converts, `ToNumber` and `ToString`. Each
setter takes any value, `impl IntoUnknown`, as `setAttribute`'s value
does: `select.set_size(value)` is `select.size = value`. Held by
`test/webapi-bindings.test.ts`, "a value the DOM makes text of takes any
value".


## Amendment: an image's `srcset`, a style's `display` and margins (rd159)

React DOM's ReactFiberConfigDOM sets `img.srcset = newProps.srcSet` in
commitMount and, restoring a view transition's name, `style.display =
display`, `style.margin = margin`, `style.marginTop` and
`style.marginBottom` of a style prop's values, any of them. Each setter
takes any value, `impl IntoUnknown`: the list's
`HTMLImageElement.set srcset` and `CSSStyleProperties.set display`,
`set margin`, `set marginTop` and `set marginBottom`. A CSS property's
setter, generated of TypeScript's `CSSStyleProperties` (`cssProperties`),
asks the same list, one catalog; the rest still take text. Converting it
in Rust first, `js::string(value)`, would differ of a symbol, which
`String(value)` makes text of and the DOM throws a `TypeError` of. Held by
`test/webapi-bindings.test.ts`'s "a value the DOM makes text of takes any
value" (`style.display = display` of a symbol, given as it is).
