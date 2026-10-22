# Emitting modules

Each Rust module is one ES module, written beside its Rust. Your module
tree is your JS file tree, and `use` between modules is `import` between
files.

## Where each module's file goes

```
src/
├─ lib.rs              mod format; mod greet; mod lint; #[path] mod slug;
├─ lib.js
├─ format.rs           pub mod units { .. } inside
├─ format.js
├─ format/units.js     the inline module, where its file would be
├─ greet.rs
├─ greet.js
├─ lint.rs
├─ lint.js
└─ routes/
   ├─ [slug].rs        #[path = "routes/[slug].rs"] pub mod slug;
   └─ [slug].js        beside the file #[path] names
```

| Rust | JS |
| --- | --- |
| the crate root, `src/lib.rs` or `[lib] path` | `src/lib.js`, `src/App.jsx` |
| `mod greet;` in `src/greet.rs` | `src/greet.js` |
| `mod widgets;` in `src/widgets/mod.rs` | `src/widgets/mod.js` |
| `mod units { .. }`, inline in `src/format.rs` | `src/format/units.js` |
| `#[path = "routes/[slug].rs"] mod slug;` | `src/routes/[slug].js` |

- **A module with JSX is `.jsx`**, any other `.js`. A root module that
  stops having JSX becomes `App.js`: import it as `./App` and restart a
  Vite dev server to see the other file.
- **A module has a file for what it has**: an item, a `js::import!`, a
  re-export or a default. A module of bindings alone writes none, and nor
  does a crate root of `mod`s alone ([ADR 0382](../../decisions/0382-a-module-has-a-file-for-what-it-has.md)).
- **`#[path]` is how you name a file no Rust module can be named**, as
  Next.js's routes are, `[slug]` ([ADR 0273](../../decisions/0273-path-modules.md)).
- **Two modules can't be one file.** A crate root `src/root.rs` with an
  inline `pub mod root { .. }` would write `src/root.js` twice, so
  `rust-js build` stops before writing anything:

  ```
  rust-js can't write the JS in source: the crate root and module `root` would both be /path/to/app/src/root.js; rename the module
  ```

## Imports between modules

A module imports what it uses from the others, by a relative path, as you'd
write it:

```rust
// src/lib.rs
pub mod format;
mod greet;
mod lint;
#[path = "routes/[slug].rs"]
pub mod slug;

pub use greet::hello;

pub fn welcome(name: &str) -> String {
    format::shout(&hello(name))
}
```

```js
// src/lib.js
import { shout } from "./format.js";
import { hello } from "./greet.js";

export { hello } from "./greet.js";

export function welcome(name) {
  return shout(hello(name));
}
```

```rust
// src/routes/[slug].rs
pub fn page() -> String {
    crate::welcome("slug")
}
```

```js
// src/routes/[slug].js
import { welcome } from "../lib.js";

export function page() {
  return welcome("slug");
}
```

Imports are one block, in the order a person writes them: packages, then
the crate's own modules, then imports for their effect, then
`@rust-js/runtime` ([ADR 0295](../../decisions/0295-import-order.md)).

## `pub` and exports

```rust
// src/lib.rs
mod format;

pub fn welcome(name: &str) -> String {
    format::shout(&greeting(name))
}

fn greeting(name: &str) -> String {
    format!("Hello, {name}")
}
```

```rust
// src/format.rs
pub(crate) fn shout(text: &str) -> String {
    text.to_uppercase() + "!"
}
```

```js
// src/lib.js
import { shout } from "./format.js";

export function welcome(name) {
  return shout(greeting(name));
}

function greeting(name) {
  return `Hello, ${name}`;
}
```

```js
// src/format.js
export function shout(text) {
  return text.toUpperCase() + "!";
}
```

```ts
// src/lib.d.ts
export function welcome(name: string): string;
```

- **A private item is the module's own**: not exported.
- **An item another module uses is exported**, so that module can import
  it: `pub(crate)` and `pub(super)` too.
- **The `.d.ts` declares what's `pub`**, your crate's API for TypeScript.
  `format.rs` has nothing `pub`, so it has no `.d.ts`.
- **Names are JavaScript's**: `total_area` is `totalArea`. See
  [JavaScript types](../interop/types.md) and
  [Export Rust APIs to JavaScript](../interop/export-to-javascript.md).

## Re-exports

A `pub use` of another module's function is `export { .. } from` it, in
the JS and the `.d.ts`; `pub use .. as` renames it.

```rust
pub use greet::hello;
pub use format::units::px as pixels;
```

```js
export { hello } from "./greet.js";
export { px as pixels } from "./format/units.js";
```

A module of only `pub use`s is a file of them, as an `index.js` is. A
`pub use` of a binding re-exports from the JS module it binds.

## The default export

`js::export_default!` names the module's default export, a function of
its own, which keeps its name too:

```rust
pub fn page() -> String {
    crate::welcome("slug")
}

js::export_default!(page);
```

```js
export function page() {
  return welcome("slug");
}

export default page;
```

Another module's function as the default is re-exported:
`js::export_default!(super::page::page);` is
`export { page as default } from "./page.js";`.

## Import a module for its effect

`js::import!` with a string imports a module for what it does when it's
loaded, a stylesheet say. Write it at the top of the module that needs it.

```rust
js::import!("./routes/slug.css");
```

```js
// src/routes/[slug].js
import "./slug.css";
```

The path, like every relative module in a `link_name`, is relative to the
crate root's JS file, so one path means one file from every module; the JS
names it from the importing file ([ADR 0028](../../decisions/0028-js-module-imports.md)).

## Load a module later

`js::import!` with a function of another module is a dynamic `import()`,
a promise of it. Nothing imports that module statically, so a bundler
splits it into a chunk of its own ([ADR 0304](../../decisions/0304-dynamic-import.md)).

```rust
pub async fn long_lines(text: String) -> usize {
    let count = js::import!(lint::long_lines).await;
    count(&text)
}
```

```js
export async function longLines(text) {
  const { longLines: count } = await import("./lint.js");
  return count(text);
}
```

`js::import_module!(root::Root)` is the module's namespace, `import("./root.js")`,
whose default export is `Root`, as React's `lazy` takes it.

## Run code when the module loads

`js::on_load!` is what the module runs when it's loaded: its body becomes
the module's top-level statements. rustc checks it as a function's body
([ADR 0267](../../decisions/0267-on-load.md)).

```rust
#[cfg_attr(rust_js, rust_js::link_name = "nanoid#nanoid")]
pub fn nanoid() -> String {
    unreachable!()
}

js::on_load! {
    println!("greet loaded");
}

pub fn hello(name: &str) -> String {
    format!("Hello, {name} #{}", nanoid())
}
```

```js
import { nanoid } from "nanoid";

console.log("greet loaded");

export function hello(name) {
  const arg = nanoid();
  return `Hello, ${name} #${arg}`;
}
```

A module's items are in the order you wrote them, each after what it reads
when it's loaded.

## Directives

`js::directive!` at a module's top is its directive, the first statement of
its JS, as a Next.js Client Component needs:

```rust
js::directive!("use client");
```

```js
"use client";
```

In a function's or a closure's body, it's that body's directive, as
Next.js's `"use server"` actions and `"use cache"` functions have:

```rust
pub async fn cached(n: u32) -> u32 {
    js::directive!("use cache");
    n + 1
}
```

```js
export async function cached(n) {
  "use cache";
  return (n + 1) >>> 0;
}
```

One in an `async` block, which is no function, is an error ([ADR 0349](../../decisions/0349-body-directives.md)).

## Libraries

A library is a crate others depend on, a workspace's `models` or a package
on npm. You build it as you build an app, `rust-js build` in its
directory, and nothing else is set: Cargo's build compiles every crate as
a library, with a manifest of what each module exports, which the crates
that use it read.

To share one on npm ([ADR 0414](../../decisions/0414-a-library-on-npm-as-rescripts-are.md)):

- **Publish the crate and its JS together**: `Cargo.toml`, the Rust, and
  each `.js` and `.d.ts` your `rust-js build` wrote. Not the maps.
- **Say which crate it has** in `package.json`, `"rust-js": { "crate":
  "acme-greet" }`, with `main` and `types` its root's files.
- **If it has `exports`, export the JS**: `"./src/*": "./src/*"`. A rust-js
  app imports `@acme/greet/src/lib.js`, and `rust-js build` refuses one
  whose `exports` hide it, saying what to export.
- **Depend on `@rust-js/runtime`** where its JS imports it, as any package
  depends on what it imports.

Its consumers:

- **A JS app** imports the JS you wrote, `import { greet } from
  "@acme/greet"`, as it imports any package.
- **A rust-js app** compiles your crate again, by its own rust-js, into the
  package where it's installed, and imports it by the package's name. Your
  crate's warnings aren't the app's; they're quiet in its build. See
  [Dependencies](dependencies.md#a-library-on-npm).
