# 0236. What JSX makes is `JSX::Element`, TypeScript's `JSX.Element`

Status: Accepted. Amends [0196](0196-typescript-declarations.md),
[0224](0224-typed-intrinsic-elements.md) and
[0227](0227-react-type-names.md).

## Context

@types/react names what JSX makes `JSX.Element`, `interface Element extends
React.ReactElement<any, any> {}` in its `JSX` namespace, and a component
that returns JSX is inferred to return one. react's crate called it
`react::Element`, which a reader takes for the DOM's, `webapi::Element`,
and its declarations said `ReactNode`, anything React renders, wider than
what JSX makes:

```ts
declare function ExpandableCallout(props: ExpandableCalloutProps): ReactNode;
```

## Decision

**What JSX makes is `react::JSX::Element`, in a module `JSX` as
TypeScript's namespace, and its declarations say `JSX.Element`, imported
as `JSX` from react.**

```rust
use react::{JSX, jsx};

pub fn Counter() -> JSX::Element {
    jsx! { <button>{"count"}</button> }
}
```

```ts
import type { JSX } from "react";

export function Counter(): JSX.Element;
```

- **`react::Element` is gone**: `JSX::Element` is its only name, beside
  `webapi::Element`, the DOM's, and `ReactElement<P>`, an element of known
  props.
- **A type a binding names of a namespace, `react#JSX.Element`, imports
  the namespace**, `JSX`, and names its member.
- **A tag's is still of its DOM element while it's built**,
  `JSX::Element<webapi::HTMLButtonElement>`, and `JSX.Element` whatever
  it is (ADR 0224).

## Why

- **It's React's name**, in Rust as in TypeScript: `JSX::Element` reads as
  `JSX.Element` does.
- **It's what the JS returns**, not anything React may render.
- **It's tested**: the declaration tests check `JSX.Element` returns and
  `import type { .., JSX, .. } from "react"`; a mutation imports the
  member whole, which they refuse.
