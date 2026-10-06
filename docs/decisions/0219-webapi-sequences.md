# 0219. A sequence a webapi function takes is a slice

Status: Accepted. Extends [0024](0024-web-crate.md) and
[0215](0215-webapi-unions.md).

## Context

WebIDL's `sequence<T>` is a JS array a function takes: `new Blob(parts)`
of `sequence<BlobPart>`, `navigator.clipboard.write(items)` of
`sequence<ClipboardItem>`. The webapi crate left out each function that
took one, and react.dev's "Copy page" button does both.

## Decision

**A sequence a function, a constructor or a dictionary takes is a slice of
its items' type, `&[BlobPart]`, a JS array of each item as it is:**

```rust
blob::new_with_blob_parts(&[text.into(), "!".into()]);   // new Blob([text, "!"])
clipboard::write(navigator::clipboard(navigator), items); // navigator.clipboard.write(items)
```

- **An item of a union is the union's enum** (ADR 0215), whose value is
  the member itself, so the array is what JS takes.
- **A sequence in a union is that union's array**, a variant `List`.
- **Not yet: one a function returns**, which would be a `Vec`, made by JS.

With it, `Navigator` (the `navigator` global), `Clipboard` and
`ClipboardItem` of clipboard-apis are bound.

## Why

- **A slice is an array in rust-js**, so the JS a call makes is
  TypeScript's, the array as it's written.
