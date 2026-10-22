# JavaScript environments: browsers, Node, Vite, Next.js

rust-js writes standard ES modules: `import` and `export`, no CommonJS, no
globals of its own. Whatever runs or bundles ES modules runs them.

| Where | How |
| --- | --- |
| A browser | through a bundler, Vite's or Next.js's, as any app's JS |
| Node.js | `node` imports the JS as it is |
| Bun | `bun` imports it too; Node is what rust-js promises |
| Vite | serves the JS `rust-js build` writes beside the Rust; no plugin |
| Next.js | serves Rust routes and components as JSX files; the `next` crate binds its API |

## Bare imports need a resolver

The JS imports packages by name: `react`, `nanoid`, and rust-js's own
`@rust-js/runtime`. A bundler or Node resolves them from `node_modules`. A
browser loading the files directly needs an import map for each.

## The runtime package

A module imports what Rust needs and JS hasn't from `@rust-js/runtime`,
one named import per helper it uses:

```rust
pub fn label(count: u32) -> String {
    format!("{count} items")
}

pub fn pick(items: &[String], index: usize) -> String {
    items[index].clone()
}
```

```js
import { $index } from "@rust-js/runtime";

export function label(count) {
  return `${count} items`;
}

export function pick(items, index) {
  return $index(items, index);
}
```

- **A helper appears where behavior needs one.** `items[index]` panics out
  of bounds, as Rust's does, so it's `$index`; `format!` of a number is a
  template literal, so it's none. Each helper starts with `$`.
- **`@rust-js/std`** holds some of std's own bodies, compiled to JS, which
  the JS imports as `@rust-js/std/iter.js` where it uses them.
- **Install both, at the compiler's version**, as `dependencies`: they're
  what the app runs. `rust-js build` refuses another version ([ADR 0103](../../decisions/0103-runtime-package.md)).
- **Each helper is a named export**, and the packages have no side
  effects, so a bundler keeps only what's used.

What each Rust construct becomes, and when it needs a helper, is
[Semantics](../semantics.md).

## Browsers

The `webapi` crate binds the web platform, generated from the same WebIDL
as TypeScript's `lib.dom.d.ts`. A call is the DOM's own:
`document.set_title(title)` is `document.title = title`.

- **Some of std needs a process**, which a browser hasn't:
  `std::env::args`, `std::env::var`, `std::env::vars` and
  `std::process::exit` work in Node and panic in a browser, saying so:

  ```
  Error: `std::env::args` needs Node's process, which a browser hasn't (ADR 0391)
  ```

- **What JavaScript has no way to do is refused at compile time**: threads,
  files through `std::fs`, raw memory. See [Compile errors](errors.md).

## Node.js and Bun

Node runs the JS as it's written:

```bash
node -e 'import("./src/lib.js").then((m) => console.log(m.totalArea([1, 2])))'
# 15.707963267948966
```

- **Node's modules are the `node` crate's**, typed as `@types/node` types
  them: `node::fs::read_file_sync` is `readFileSync` from `"fs"`. See
  [Dependencies](dependencies.md#use-one).
- **`std::env` and `std::process::exit` are Node's process**:
  `env::args()` is `process.argv.slice(1)`, `process::exit(code)` is
  `process.exit(code)`.
- **`println!` is `console.log`**, and `print!` is `process.stdout.write`.
- **Node is what rust-js's JS is tested on**, and what it promises. Bun
  imports the same JS, and `bun test` runs rust-js's tests ([ADR 0095](../../decisions/0095-node-runtime.md)).
- **Give your package `"type": "module"`**, so Node loads the `.js` files
  as ES modules without reparsing them.

## Vite

A Vite app needs nothing of rust-js in `vite.config.js`. Vite serves the
`.jsx` and `.js` files `rust-js build` writes as any of the app's own, and
React's plugin gives them Fast Refresh.

```bash
bun create @rust-js@latest my-app     # a Vite and React app, its component src/App.rs
```

```js
// src/main.jsx
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./index.css";
import { App } from "./App.jsx";

createRoot(document.getElementById("root")).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
```

- **Import by name**: rust-js exports each function by its name, `{ App }`.
- **Run `rust-js build` beside `vite`**, after each change to the Rust. A
  compile error leaves the page as it was. See
  [Building](building.md#run-it-beside-your-dev-server).
- **`vite build` builds what's committed**: put `rust-js build --check`
  first in the `build` script.
- **Tailwind CSS reads the JSX rust-js writes.** Leave the Rust out of its
  sources, so an edit to a `.rs` file before it's built doesn't reload the
  page:

  ```css
  @import "tailwindcss";
  @source not "./**/*.rs";
  ```

  Write each class name whole, as in any template: `if even {
  "text-emerald-500" } else { "text-sky-500" }`.
- **A root module that stops having JSX** becomes `App.js`: import it as
  `./App`, and restart Vite to see the other file.

The walk-through is [Getting started with rust-js and
Vite](../tutorial/vite.md).

## Next.js

A Next.js app's routes and components can be Rust, each compiled to the
JSX file Next.js looks for beside it. The `next` crate binds Next.js's
components and hooks.

```bash
bun create @rust-js@latest my-site --template next
```

```toml
[lib]
path = "app/page.rs"

[dependencies]
js = { package = "rust-js-builtins", version = "~0.0.2" }
next = { package = "rust-js-next", version = "~0.0.4" }
react = { package = "rust-js-react", version = "~0.0.4" }
```

```rust
// app/page.rs
use next::link::Link;
use react::{JSX, jsx};

pub mod about {
    pub mod page;
}

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

```rust
// app/about/page.rs
js::directive!("use client");

use react::{JSX, jsx, use_state};

pub fn About() -> JSX::Element {
    let (likes, set_likes) = use_state(0);
    jsx! {
        <button onClick={move |_| set_likes.update(|likes| likes + 1)}>
            {"Likes: "}{likes}
        </button>
    }
}

js::export_default!(About);
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

```jsx
// app/about/page.jsx
"use client";

import { useState } from "react";

export function About() {
  const [likes, setLikes] = useState(0);
  return <button onClick={() => setLikes((likes) => (likes + 1) | 0)}>Likes: {likes}</button>;
}

export default About;
```

- **A route is a module where Next.js looks for its file**:
  `app/about/page.rs`, `mod about { pub mod page; }`, is
  `app/about/page.jsx`. A segment no Rust module can be named is a
  `#[path]` module: `#[path = "[slug]/page.rs"] pub mod post;` in
  `app/page.rs` is `app/[slug]/page.jsx`.
- **A page is its module's default export**, `js::export_default!`.
- **A component is a Server Component** unless its module says
  `js::directive!("use client");`, as one with state or events must.
- **Pages Router routes get no `.d.ts`**: Next.js would take one for a
  route ([ADR 0276](../../decisions/0276-no-declarations-in-routes.md)).
- **The app's other files stay JS**: `next.config.js`, `layout.js`,
  middleware. Rust and JS modules import each other.
- **One crate is the server's and the browser's.** What needs Node, `node`'s
  modules or `std::env`, works where the code runs on the server, and
  panics in a browser.
- **`next build` builds what's committed**: `"build": "rust-js build
  --check && next build"`.

What the `next` crate has, and data fetching, actions and metadata, is
[Next.js](../react/nextjs.md).
