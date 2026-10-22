# Getting started with rust-js and Vite

You'll make a Vite and React app whose component is written in Rust, run
it, change it, and build it for production. On the way you'll see the
JavaScript rust-js writes from your Rust.

Next: [Build UIs with React](react.md), then
[Use a JavaScript library from Rust](js-library.md).

## What you need

Install each of these once:

| What | Why | How |
| --- | --- | --- |
| [Bun](https://bun.sh) or [Node.js](https://nodejs.org) | makes the app, installs its packages, runs Vite | Bun: `curl -fsSL https://bun.sh/install \| bash`. Node.js must be `^20.19.0` or `>=22.12.0`, the versions Vite 8 runs on |
| [rustup](https://rustup.rs) | installs Rust toolchains | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| rust-js's Rust | rust-js runs with the rustc it was built with | `rustup toolchain install 1.99.0 --profile minimal --target wasm32-unknown-unknown` |

- **The Rust release is rust-js's, not yours.** rust-js uses rustc's own
  library, so it needs the exact release it was built with. rustup keeps it
  apart from the Rust you already have. It's about 500 MB.
- **`wasm32-unknown-unknown` is the target rustc checks your app for.** Its
  `usize` is 32 bits, as rust-js's is. Nothing is compiled to WebAssembly.
- rust-js runs on macOS and Linux.

## Make an app

```bash
bun create @rust-js@latest my-app      # with Bun
npm create @rust-js@latest my-app      # with Node.js
```

It makes `my-app`, a directory that must be new or empty. It prints the
`rustup` command above, to run once, and then the commands to run:

```bash
cd my-app
bun install          # or npm install
bun run dev          # or npm run dev: http://localhost:5173
```

Open http://localhost:5173. The page is create-vite's React template, with
its component written in Rust.

## What's in it

```
my-app/
├─ Cargo.toml          the app's crate: its Rust is src/App.rs
├─ package.json        Vite, React, and rust-js's packages
├─ vite.config.js      React, React Compiler, Tailwind CSS
├─ index.html
├─ .vscode/settings.json   rust-analyzer checks the app through rust-js
└─ src/
   ├─ App.rs           your component, in Rust
   ├─ App.jsx          what rust-js writes from it: committed
   ├─ App.d.ts         its TypeScript declarations: committed
   ├─ App.jsx.map      its source map: not committed
   ├─ main.jsx         mounts <App />, as any Vite app does
   ├─ App.css, index.css
   └─ assets/
```

- **The app is a Cargo package.** `Cargo.toml` names the crate's root,
  `src/App.rs`, and the crates it uses: `js`, `webapi` and `react`.
  Those crates come from npm, as `@rust-js/builtins`, `@rust-js/webapi` and
  `@rust-js/react` in `package.json`. `Cargo.toml` names them by version,
  and `rust-js build` tells Cargo they're in `node_modules`, in a patch it
  writes to `.cargo/config.toml` each time it runs.
- **`[lints.rust]` lists `cfg(rust_js)`.** rust-js compiles with
  `--cfg rust_js`, and its attributes are written
  `#[cfg_attr(rust_js, ..)]` so a plain `cargo check` compiles the app too
  ([ADR 0113](../../decisions/0113-plain-rustc.md)). Keep the entry.
- **Each Rust module's JS is written beside it.** `src/App.rs` is
  `src/App.jsx`, a module with JSX in it, and a module without JSX is
  `.js`: `src/greeting.rs` is `src/greeting.js`.
- **The JS is committed**, as ReScript recommends for its JS: a diff shows
  what a change did to the output, and a checkout without rust-js still
  builds ([ADR 0041](../../decisions/0041-react.md)). Its source maps aren't: the template's `.gitignore`
  leaves out `src/App.jsx.map`.
- **What Cargo writes isn't committed**: `/target`, `/Cargo.lock` and
  `/.cargo` are in `.gitignore`. Cargo builds in
  `node_modules/.cache/rust-js/target`, out of Vite's way.

## Write your first program

Replace `src/App.rs` with a component, and add a module beside it.
rust-js writes each module's JS beside it, and its declarations for
TypeScript code that imports it: `src/App.jsx`, `src/greeting.js` and
`src/greeting.d.ts`, among them.

```rust
// src/App.rs
mod greeting;

use greeting::greet;
use react::{JSX, jsx, use_state};

pub fn App() -> JSX::Element {
    let (count, set_count) = use_state(|| 0);

    jsx! {
        <main>
            <h1>{greet("Vite")}</h1>
            <button type="button" onClick={move |_| set_count.update(|count| count + 1)}>
                {"Count is "}{count}
            </button>
        </main>
    }
}
```

```rust
// src/greeting.rs
/// A greeting for `name`.
pub fn greet(name: &str) -> String {
    format!("Hello, {name}!")
}
```

```jsx
// src/App.jsx
import { useState } from "react";
import { greet } from "./greeting.js";

export function App() {
  const [count, setCount] = useState(0);

  return (
    <main>
      <h1>{greet("Vite")}</h1>
      <button type="button" onClick={() => setCount((count) => (count + 1) | 0)}>
        Count is {count}
      </button>
    </main>
  );
}
```

```js
// src/greeting.js
/** A greeting for `name`. */
export function greet(name) {
  return `Hello, ${name}!`;
}
```

```ts
// src/greeting.d.ts
/** A greeting for `name`. */
export function greet(name: string): string;
```

To write them, run `rust-js build` in a second terminal, in `my-app`:

```bash
bunx rust-js build     # or npx rust-js build
```

- **`pub fn` is `export function`.** A component is a `pub fn` with a
  capitalized name that returns `JSX::Element`.
- **`mod greeting;` is an ES module**, and `use greeting::greet` its
  import.
- **Doc comments are JSDoc.** `///` is `/** */`.
- **`| 0` keeps an `i32` an `i32`.** Rust's integers wrap at their width;
  rust-js writes what keeps them so. See
  [Semantics](../semantics.md).
- **Names are camelCase in JS.** `set_count` is `setCount`.

Each file also starts with `// Generated by rust-js from src/App.rs. Do not
edit.` and ends with its `//# sourceMappingURL=` line, left out here.

## The edit loop

Vite and `rust-js build` are two processes, each its own ([ADR 0412](../../decisions/0412-rust-js-build-is-its-own-process.md)):

```
edit App.rs ─► rust-js build ─► App.jsx ─► Vite HMR ─► Fast Refresh keeps state
```

- **`bun run dev` runs Vite**, as in any Vite app. Leave it running.
- **`bunx rust-js build` compiles the Rust once**, each time you run it,
  after you save. Vite sees the new JS as an edit, and Fast Refresh keeps
  the component's state.
- **A compile error is rustc's**, in the terminal you ran `rust-js build`
  in, and nothing is written. The page keeps running the last version that
  compiled:

```text
cargo check failed:
error[E0308]: mismatched types
  --> src/App.rs:11:24
   |
11 |             <h1>{greet(42)}</h1>
   |                  ----- ^^ expected `&str`, found integer
   |                  |
   |                  arguments to this function are incorrect
```

- **The browser's source maps point at your Rust.** DevTools shows
  `App.rs`, and a breakpoint there stops in the handler you wrote.
- **Your editor checks the app through rust-js.** `.vscode/settings.json`
  sets rust-analyzer's check to `./node_modules/.bin/rust-js-check`, which
  reads inside `jsx!` as a plain `cargo check` can't ([ADR 0222](../../decisions/0222-editor-check.md)).

See [Compile errors](../project/errors.md) for more of what rustc and
rust-js say.

## Build for production

```bash
bun run build          # or npm run build: writes dist/
```

The template's `build` script is `rust-js build --check && vite build`.
`rust-js build --check` writes nothing; it fails if a module's JS isn't
what its Rust compiles to, and names each one:

```text
rust-js: the JS beside the Rust isn't what it compiles to; run `rust-js build`:
  src/greeting.js
```

Run `bunx rust-js build`, then `bun run build` again. `vite build` builds
the committed JS as it builds any JS. Run the same check in CI.

`bun run preview` serves `dist/`.

## Next

- [Build UIs with React](react.md): components, props, hooks and events.
- [Use a JavaScript library from Rust](js-library.md): an npm package, bound
  and called.
- [Building the application](../project/building.md): `rust-js build`,
  workspaces and CI.
- [JavaScript environments](../project/environments.md): browsers, Node,
  Vite and Next.js.
