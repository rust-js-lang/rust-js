# 0350. Next.js's metadata is typed as Next.js types it, its tagged unions one struct

Status: Accepted. Part of [0346](0346-next-coverage.md); uses
[0214](0214-untagged-enums.md)'s untagged enums.

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

An App Router page's or layout's `<head>` is its `metadata`, or what its
`generateMetadata` gives: Next.js's `Metadata`, forty fields, most a union,
`title?: string | TemplateString`, `icons?: IconURL | Icon[] | Icons`.
`openGraph` is a union of thirteen objects by their `type`, each the same
fifteen fields and a few of its own; `twitter`, of five by their `card`.
`app/robots`, `app/sitemap` and `app/manifest` give `MetadataRoute`'s.

## Decision

**Each type is a struct of Next.js's fields, each `None` but what's given,
and each union an untagged enum of its members, as webapi's are (ADR
0215).** `generateMetadata` gives the object JS would write:

```rust
Metadata {
    title: Some(Title::Str("About")),
    open_graph: Some(OpenGraph { r#type: Some("article"), published_time: Some("2026-10-10"), ..Default::default() }),
    ..Default::default()
}
```

```js
return { title: "About", openGraph: { type: "article", publishedTime: "2026-10-10" } };
```

- **`OpenGraph` and `Twitter` are each one struct of every member's
  fields**, `type` and `card` among them. A tagged enum of thirteen
  variants (ADR 0284) would repeat the fifteen shared fields in each, and
  a variant can't be made `..Default::default()`; a variant's flattened
  field, which would share them, isn't made yet (ADR 0204). Each value
  TypeScript takes is one, the same object; one it refuses, a book's
  `actors`, isn't refused.
- **`T | T[]` is `OneOrMany<T>`**, the crate's one enum of it, the proxy
  config's `matcher` too.
- **A string literal union is `&str`**, as React's attributes are.
- **`null`, which clears what a layout set, isn't one yet**: a field's
  `None` is left out, which a page inherits.
- **`MetadataRoute` is a module**, its `Robots`, `Sitemap` and `Manifest`,
  as TypeScript's namespace is.
- **`ResolvedMetadata`, what `generateMetadata` is given of its layouts,
  is read by its fields**, `title`, `description`, `openGraph`, and any by
  its name.

## Why

- **It's the object Next.js reads**: the test's page's `<title>`, its
  `<meta>`s, `robots.txt` and `sitemap.xml` are Next.js's of it.
- **It's how a page writes it**, a literal of what it sets.

## Consequences

- `test/next.test.ts` builds a page of a `generateMetadata` and a
  `generateViewport`, and an `app/robots.rs` and an `app/sitemap.rs`, and
  checks what Next.js made of them.
