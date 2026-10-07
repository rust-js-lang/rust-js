# next: Next.js for rust-js

Write a Next.js app's routes and components in Rust. rust-js compiles each
to the JSX you'd write beside it, `app/page.rs` to `app/page.jsx`, and
`rust-js-next`, `@rust-js/next-plugin`'s, runs it beside `next dev` and
before `next build` ([ADR 0192](../docs/decisions/0192-next.md)):

```rust
#![allow(non_snake_case)]

use next::link::Link;
use react::{JSX, jsx};

pub fn Home() -> JSX::Element {
    jsx! {
        <main>
            <h1>{"Hello, Next.js"}</h1>
            <Link href="/about" {..Default::default()}>{"About"}</Link>
        </main>
    }
}

js::export_default!(Home);
```

```jsx
// app/page.jsx
import Link from "next/link";

export function Home() {
  return (
    <main>
      <h1>Hello, Next.js</h1>
      <Link href="/about">About</Link>
    </main>
  );
}

export default Home;
```

A module is a Server Component unless it says `js::directive!("use client");`,
as one with state or events must. The crate binds `next/image`'s `Image`,
`next/link`'s `Link`, and `next/navigation`'s `use_router`, `use_pathname`,
`use_search_params`, `not_found` and `redirect`, and the Pages Router's
`next/router` `use_router`, whose `as_path` react.dev reads: a component's optional
props are `None` unless they're given, the rest from `{..Default::default()}`.

`bun create @rust-js my-site --template next` makes an app that uses it.
