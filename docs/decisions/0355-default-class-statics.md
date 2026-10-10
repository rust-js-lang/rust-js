# 0355. A default class's statics name its import after the class

Status: Accepted. Part of [0346](0346-next-coverage.md).

## Context

A default import is named after the item that holds it: `fn Image` of
`next/image#default` is `import Image`, `static Router` is `import Router`.
`next/app#default.getInitialProps` is a static of the default export,
`App`, held by `fn get_initial_props` in `mod app`. Named after that
function, it was `import orig_get_initial_props from "next/app"` and
`orig_get_initial_props.origGetInitialProps(context)`, or `app` where two
were used. Next.js's docs write `App.getInitialProps`.

## Decision

**An item names a default import only when it holds the export itself,
`next/app#default`; where every item holds one of its members, a class's
statics in a module named for it, `mod app`, the import is the class's
name, the module's in upper camel case, `App`:**

```js
import App from "next/app";

return await App.origGetInitialProps(context);
```

- A module of a bindings crate counts, as one of the program's does for a
  namespace (ADR 0256).
- A default that is itself bound, next/dynamic's `dynamic`, keeps its
  name, even beside its members.

## Consequences

- `next/document#default.getInitialProps` is `Document.getInitialProps`.
