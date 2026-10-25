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

  Nor can two whose files differ only by case, `mod shapes { .. }` and
  `mod Shapes { .. }`: `shapes.js` and `Shapes.js` are one file on macOS
  and Windows, so both are refused, naming the two files.

## Imports between modules

A module imports what it uses from the others, by a relative path, as you'd
write it:

```rust
// src/lib.rs
pub mod format;
mod greet;
#[path = "routes/[slug].rs"]
pub mod slug;

pub use greet::hello;

pub fn welcome(name: &str) -> String {
    format::shout(&hello(name))
}
```

```rust
// src/format.rs
pub fn shout(text: &str) -> String {
    text.to_uppercase() + "!"
}
```

```rust
// src/greet.rs
pub fn hello(name: &str) -> String {
    format!("Hello, {name}")
}
```

```rust
// src/routes/[slug].rs
pub fn page() -> String {
    crate::welcome("slug")
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

A `use .. as` names the import as it renames the item, camelCase as any
name is: another module's function, a library's item, a module's default
too, `import queueMacrotask from "../shared/enqueueTask.js"`, and a
binding's export, `import { useId as makeId } from "react"`; by its
`#[rust_js::name]` where it has one. The module reads it by that name;
every other module keeps its own. Two `use`s naming one item apart in
one module are an error, as its file imports it once
([ADR 0202](../../decisions/0202-imports-named-around-importers.md),
amended):

```rust
// src/lib.rs
mod format;

use format::shout as loud;

pub fn welcome(name: &str) -> String {
    loud(name)
}
```

```rust
// src/format.rs
pub fn shout(text: &str) -> String {
    text.to_uppercase() + "!"
}
```

```js
// src/lib.js
import { shout as loud } from "./format.js";

export function welcome(name) {
  return loud(name);
}
```

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

A `pub use` of another module's function or constant is `export { .. }
from` it, in the JS and the `.d.ts`. `pub use .. as` renames it, camelCase
as any name is, or by its `#[rust_js::name]`. Each `pub use` is written
where it is, in the order written, with the comments before it; a run of
them from one module with no comment or blank line between, their
attributes aside, is one `export { .. } from`
([ADR 0874](../../decisions/0874-a-modules-pub-uses-are-re-exported-as-written.md)). One of the module's own
functions or constants by another name is exported by that name alone:
it's declared without `export`, and one `export { .. as .. }` after the
module's code lists each such name, in the order the `pub use`s are
written. The module still reads it by its own name. Every importer
imports it by the name it's exported as: another module of the crate,
another crate and a `pub use` of it. A module that names it by its own
path reads it by its own name, `import { unstable_greet as greetAll }`;
one whose `use` names the alias reads the alias
([ADR 0596](../../decisions/0596-a-pub-use-of-a-modules-own-item-exports-it.md)).
The item stays `pub` in Rust, as a `pub use` of a private item is an
error (E0364). A module can't list an item by its own name this way:
`pub use self::to_array;` is an error too (E0255), so such a function is
`export function toArray`.

```rust
// src/lib.rs
# mod format;
# mod greet;
#
pub use format::units::{px as pixels, BASE};
pub use greet::hello;
#[cfg_attr(rust_js, rust_js::name = "unstable_greet")]
pub use self::greet_all as unstable_greet;

pub fn greet_all(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| hello(name)).collect()
}
```

```rust
// src/greet.rs
pub fn hello(name: &str) -> String {
    format!("Hello, {name}")
}
```

```rust
// src/format.rs
pub mod units {
    pub const BASE: u32 = 16;

    pub fn px(n: u32) -> String {
        format!("{n}px")
    }
}
```

```js
import { hello } from "./greet.js";

export { px as pixels, BASE } from "./format/units.js";
export { hello } from "./greet.js";

function greetAll(names) {
  return names.map((name) => hello(name));
}

export { greetAll as unstable_greet };
```

```ts
export { px as pixels, BASE } from "./format/units.js";

export { hello } from "./greet.js";

declare function greetAll(names: string[]): string[];

export { greetAll as unstable_greet };
```

What a `pub use` names is re-exported from the module its path names,
which may re-export it itself, by the name it has there; a library's
item, its default or a binding a module of it holds, from the library's
file; a class, by its constructor; a module, its namespace, `export * as
ns from`, as is a JS module's, `#*`. A private `use .. as`'s
`#[rust_js::name]` is its import's name. One that has JS but can't be
re-exported yet, `pub use m::*;`, or std's function, is refused, never
left out
([ADR 0874](../../decisions/0874-a-modules-pub-uses-are-re-exported-as-written.md)):

```rust
// src/lib.rs
mod cache;
mod hooks;
pub mod runtime;
mod server;

// Hooks
pub use hooks::use_id;
#[cfg_attr(rust_js, rust_js::name = "unstable_useCacheRefresh")]
pub use hooks::use_cache_refresh as unstable_use_cache_refresh;
#[cfg_attr(rust_js, rust_js::name = "__COMPILER_RUNTIME")]
pub use crate::runtime as __COMPILER_RUNTIME;
// DEV-only
pub use server::cache;

#[cfg_attr(rust_js, rust_js::name = "_jsxDEV")]
use runtime::jsx_dev as _jsx_dev;

pub fn element() -> u32 {
    _jsx_dev(1)
}
```

```rust
// src/cache.rs
pub fn cache(n: u32) -> u32 {
    n
}
```

```rust
// src/hooks.rs
pub fn use_id() -> u32 {
    1
}

pub fn use_cache_refresh() -> u32 {
    2
}
```

```rust
// src/runtime.rs
pub fn jsx_dev(n: u32) -> u32 {
    n + 1
}
```

```rust
// src/server.rs
pub use crate::cache::cache;
```

```js
import { jsxDev as _jsxDEV } from "./runtime.js";

// Hooks
export { useId, useCacheRefresh as unstable_useCacheRefresh } from "./hooks.js";
export * as __COMPILER_RUNTIME from "./runtime.js";
// DEV-only
export { cache } from "./server.js";

export function element() {
  return _jsxDEV(1);
}
```

A module of only `pub use`s is a file of them, as an `index.js` is. A
`pub use` of a binding re-exports from the JS module it binds; of a
global's, `Object.keys`, it's the module's `const`, the JS function as
it is, declared as the global's own type
([ADR 0622](../../decisions/0622-a-global-binding-a-module-names-is-its-const.md)):

```rust
pub use js::object::keys;
```

```js
export const keys = Object.keys;
```

```ts
export const keys: typeof Object.keys;
```

## The default export

`js::export_default!` names the module's default export, a function of
its own, which keeps its name for the module's own code; it's exported as
the default alone, a `pub` one too, and every importer imports it so
([ADR 0611](../../decisions/0611-a-default-export-is-where-its-macro-is.md),
amended):

```rust
pub fn page() -> String {
    String::from("Hello, slug")
}

js::export_default!(page);
```

```js
function page() {
  return "Hello, slug";
}

export default page;
```

Of the function itself, `js::export_default! { .. }`, it's the function's
declaration as the default, which nothing else exports by name, a `pub`
one's neither, in its `.d.ts` as in its JS, `export default function
noop(): void;`
([ADR 0611](../../decisions/0611-a-default-export-is-where-its-macro-is.md)):

```rust
js::export_default! {
    fn noop() {}
}
```

```js
export default function noop() {}
```

Of a `const` itself, its value is the default, JS having no declaration
of a constant as one, and another module reads it as its default; one its
own module reads is declared, then exported, `const REACT_VERSION =
"19.3.0"; export default REACT_VERSION;`
([ADR 0625](../../decisions/0625-a-default-constant-is-its-value.md)):

```rust
js::export_default! {
    const REACT_VERSION: &str = "19.3.0";
}
```

```js
export default "19.3.0";
```

Whichever form makes it, a module's default is imported as its default
wherever it's named, in its crate and in another: `use
shared::noop::noop;` is `import noop from "../../shared/noop.js";`, and a
`pub use` of it re-exports the default, `export { default as noop } from
"./noop.js";`
([ADR 0773](../../decisions/0773-a-held-default-is-exported-once.md)).
A constant the default holds that a `pub use` of its module renames is
declared, then exported, as one the module reads is.

Another module's function as the default is re-exported:
`js::export_default!(super::page::page);` is
`export { page as default } from "./page.js";`. A binding the module
`use`s, a global's function or a binding's static, is its `const`, as a
`pub use` of one is
([ADRs 0622](../../decisions/0622-a-global-binding-a-module-names-is-its-const.md),
[0633](../../decisions/0633-a-binding-named-through-a-module-is-read-through-it.md)).
A static's `const` is named as any item is, by its `#[rust_js::name]`
where JS's name isn't its Rust one; what its path needs is imported,
`react#*` the module's namespace.

A module that names a binding through a module that holds it, its
default or a `pub use`, reads it through that module, as it reads a
function: it imports that module's default, or the name it exports, and
calls it by that name, with the binding's own arguments, as upstream's
`assign(..)` is. One that names the binding itself, `js::object::assign`,
reads it in place:

```rust
// src/lib.rs
pub mod assign;
pub mod internals;
pub mod reconciler;

pub struct Internals(core::marker::PhantomData<js::JsObject>);

unsafe extern "Rust" {
    #[cfg_attr(rust_js, rust_js::name = "ReactSharedInternals")]
    #[link_name = "react#*.__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE"]
    pub safe static REACT_SHARED_INTERNALS: &'static Internals;
}
```

```rust
// src/assign.rs
pub(crate) use js::object::assign;

js::export_default!(assign);
```

```rust
// src/internals.rs
pub(crate) use crate::REACT_SHARED_INTERNALS;

js::export_default!(REACT_SHARED_INTERNALS);
```

```rust
// src/reconciler.rs
use crate::Internals;
use crate::assign::assign;
use crate::internals::REACT_SHARED_INTERNALS;
use js::Unknown;

pub fn merged<'a>(target: &'a Unknown, source: &Unknown) -> &'a Unknown {
    assign(target, (source,))
}

pub fn internals() -> &'static Internals {
    REACT_SHARED_INTERNALS
}

pub fn direct<'a>(target: &'a Unknown, source: &Unknown) -> &'a Unknown {
    js::object::assign(target, (source,))
}
```

```js
// src/assign.js
const assign = Object.assign;

export default assign;
```

```js
// src/internals.js
import * as react from "react";

const ReactSharedInternals = react.__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE;

export default ReactSharedInternals;
```

```js
// src/reconciler.js
import assign from "./assign.js";
import ReactSharedInternals from "./internals.js";

export function merged(target, source) {
  return assign(target, source);
}

export function internals() {
  return ReactSharedInternals;
}

export function direct(target, source) {
  return Object.assign(target, source);
}
```

A `pub use` of a binding through a module that holds it is that module's,
re-exported, `export { keys } from "./keys.js";`; across crates, a
library's manifest says what each of its modules holds, which a crate
that names a binding through one imports from the library's JS. A
library's module is named by a crate that uses it only where it's `pub`,
`pub use js::object::assign; js::export_default!(assign);`, and what's
its default is exported as the default alone, as every caller reads it,
`import assign from "shared/assign.js"`
([ADR 0773](../../decisions/0773-a-held-default-is-exported-once.md)).

## Import a module for its effect

`js::import!` with a string imports a module for what it does when it's
loaded, a stylesheet say. Write it at the top of the module that needs it.

```rust
// src/lib.rs
#[path = "routes/[slug].rs"]
pub mod slug;
```

```rust
// src/routes/[slug].rs
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
// src/lib.rs
# mod lint;
#
pub async fn long_lines(text: String) -> usize {
    let count = js::import!(lint::long_lines).await;
    count(&text)
}
```

```rust
// src/lint.rs
pub fn long_lines(text: &str) -> usize {
    text.lines().filter(|line| line.len() > 80).count()
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

pub fn label(likes: u32) -> String {
    format!("Likes: {likes}")
}
```

```js
"use client";

export function label(likes) {
  return `Likes: ${likes}`;
}
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

## Comments and blank lines

Every comment you write is in the JS where its code is, and so is each
blank line between statements: one on its own lines is before the
statement after it, one after code on its line is after that code's JS,
its line ended there where JS has the next line's code on it, `? "…" //
ellipsis` of a conditional ([ADR
0733](../../decisions/0733-a-line-comment-ends-the-line-of-its-code.md)),
one inside an expression after the code right before it, `37 /* "%" */`
([ADR 0630](../../decisions/0630-a-comment-inside-an-expression.md)),
and a `///` doc comment is a JSDoc an editor shows a JS caller. A run of
comment lines before a `match` arm is one, the arm's; a comment before a
`let` the JS has no statement of, its value read where it's named, `let arg
= unsafe { js::cast(arg) };`, is the next statement's in its block; one in
the closure `catch_unwind` calls is in the `try`; one before a binary's
right operand, an or-pattern's alternative, `// c\n | Kind::Object(_)`,
or a let-chain's `&& let`, is before the operand the JS has of it, after
its operator; and a comment before what the JS reads away, an arm of
`prev?.types`, is before the statement it's in, never lost ([ADR 0444](../../decisions/0444-a-comment-is-where-its-code-is.md),
amended). A file's
leading comments, a license's, are its JS's first
([ADR 0383](../../decisions/0383-a-files-leading-comments-are-its-jss.md),
[ADR 0444](../../decisions/0444-a-comment-is-where-its-code-is.md)).

A doc comment's JSDoc keeps its lines: `///` lines are `/** one */`, or
`/**\n * a\n * b\n */` of more, and a `/** */` block is as written, so
`/**\n * One line.\n */` stays three. A doc comment documents an item:
before a statement or a `js::on_load!`, rustc warns that it documents
nothing (`unused_doc_comments`), and with a blank line after it clippy
does (`empty_line_after_doc_comments`); write a plain comment there, `//`
or `/* */` as rustc says, which the JS keeps as written ([ADR
0444](../../decisions/0444-a-comment-is-where-its-code-is.md)).

A blank line is where your Rust has one between the code of the JS's
lines, and nowhere else: two functions, or a `static` and the
`js::on_load!` it's frozen in, written right below each other are right
below each other in the JS, and one apart is apart, a type with no JS
between too. An object's fields are apart where the struct literal's are;
a `let` with no JS of its own, an erased cast's or a guard's whose drop
is a `finally`, is no code between a blank line and the statement after
it, the guard's `try`. What the Rust can't tell of, what
rust-js writes of no code of its own or of one line, a derive's
functions, is set apart as a person sets it ([ADR
0865](../../decisions/0865-a-blank-line-is-where-the-rust-has-one.md)).

A comment written on a struct literal's field is the field's, after its
value on its line or on lines of their own before it, and a field that's
`None`, which JS leaves out, keeps its key for it, `v: undefined, // why`
([ADR 0280](../../decisions/0280-undefined-fields-left-out.md), amended).
One before an `unsafe` block is its code's
([ADR 0444](../../decisions/0444-a-comment-is-where-its-code-is.md), amended):

```rust
use std::cell::Cell;

pub struct CacheNode {
    pub status: Cell<u32>,
    pub value: Cell<Option<f64>>,
    pub next: Cell<Option<f64>>,
}

pub fn create_cache_node() -> CacheNode {
    CacheNode {
        status: Cell::new(0), // whether it returned or threw
        value: Cell::new(None), // what it returned, or threw
        next: Cell::new(None),
    }
}

pub fn first(values: &[f64]) -> f64 {
    // Every caller gives one.
    unsafe { *values.get_unchecked(0) }
}
```

```js
export function createCacheNode() {
  return {
    status: 0, // whether it returned or threw
    value: undefined, // what it returned, or threw
  };
}

export function first(values) {
  // Every caller gives one.
  return values[0];
}
```

A comment before a macro's opening, `thread_local! {`, is its code's,
its first item's, as one in it before the `fn` it writes, `js::export_default! {
// .. fn f() {} }`, is ([ADR
0735](../../decisions/0735-a-comment-in-a-macro-before-its-fn-is-the-fns.md)), and one after a module's last code, at its file's end
or before an inline module's `}`, ends its JS
([ADR 0621](../../decisions/0621-comments-around-macros-and-at-a-modules-end.md)):

```rust
use std::cell::Cell;

// The counts.

thread_local! {
    /// How many there are.
    static COUNT: Cell<u32> = const { Cell::new(0) };
}

pub fn next_count() -> u32 {
    let n = COUNT.get() + 1; // One more.

    COUNT.set(n);
    n
}

// Keep this file's counts in step with the server's.
```

```js
// The counts.

/** How many there are. */
let count = 0;

export function nextCount() {
  const n = (count + 1) >>> 0; // One more.

  count = n;
  return n;
}

// Keep this file's counts in step with the server's.
```

A comment before a branch's value is before it in the conditional the
`if` is, as Prettier lays one out
([ADR 0658](../../decisions/0658-a-comment-before-a-branchs-value-is-before-it-in-the-conditional.md)):

```rust
pub fn limit(fast: bool) -> u32 {
    if fast {
        // What a fast host can take.
        64
    } else {
        // Room for a slow one.
        8
    }
}
```

```js
export function limit(fast) {
  return fast
    ? // What a fast host can take.
      64
    : // Room for a slow one.
      8;
}
```

So is a comment before a call's argument, or before an object literal's
field: before it, on lines of their own
([ADR 0708](../../decisions/0708-a-comment-before-an-argument-is-before-it.md)):

```rust
fn at_least(value: f64, low: f64) -> f64 {
    if value < low { low } else { value }
}

pub fn area(width: f64, height: f64) -> f64 {
    at_least(
        width * height,
        // Never below zero.
        0.0,
    )
}
```

```js
function atLeast(value, low) {
  return value < low ? low : value;
}

export function area(width, height) {
  return atLeast(
    width * height,
    // Never below zero.
    0,
  );
}
```

And one before a binary's right operand, where rustfmt writes its
operator first, is before the operand in JS, after the operator, where
JS writes it last
([ADR 0444](../../decisions/0444-a-comment-is-where-its-code-is.md), amended):

```rust
pub fn total(price: f64, count: f64, shipping: f64) -> f64 {
    price * count
        // Once an order, not an item.
        + shipping
}
```

```js
export function total(price, count, shipping) {
  return (
    price * count +
    // Once an order, not an item.
    shipping
  );
}
```

A function marked `#[inline(never)]` is Closure Compiler's `@noinline`, its
JSDoc's last line where it has one, and rust-js keeps it a function of its
own, as React keeps `testStringCoercion` out of line for its stack trace.
So is one written in a body: a frame a stack must show, which upstream JS
writes as a function called where it's made, `(() => Error(..))()`, is a
function in the body marked `#[inline(never)]`, as an applied closure is
its body ([ADR 0581](../../decisions/0581-a-closure-called-as-a-statement-is-its-body.md))
and clippy refuses one (`redundant_closure_call`).
`#[inline]` and `#[inline(always)]` say nothing: a JS engine decides
([ADR 0649](../../decisions/0649-inline-never-is-noinline.md)):

```rust
#[inline(never)]
fn test_string_coercion(value: f64) -> String {
    format!("{value}")
}

/// The value's text.
#[inline(never)]
pub fn coerced(value: f64) -> String {
    test_string_coercion(value)
}

#[inline(never)]
pub fn unknown_owner() -> &'static js::JsError {
    #[inline(never)]
    fn react_stack_top_frame() -> &'static js::JsError {
        js::js_error::new("react-stack-top-frame")
    }
    react_stack_top_frame()
}
```

```js
import { $displayF64 } from "@rust-js/runtime";

/** @noinline */
function testStringCoercion(value) {
  return $displayF64(value);
}

/**
 * The value's text.
 * @noinline
 */
export function coerced(value) {
  return testStringCoercion(value);
}

/** @noinline */
export function unknownOwner() {
  /** @noinline */
  function reactStackTopFrame() {
    return new Error("react-stack-top-frame");
  }
  return reactStackTopFrame();
}
```

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
