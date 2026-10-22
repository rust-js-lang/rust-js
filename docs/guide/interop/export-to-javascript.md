# Export Rust APIs to JavaScript

Each Rust module is an ES module. What you make `pub` is what it exports,
and JavaScript calls it as it would any other module. This page says what
a JS caller sees: names, parameters, results, types and their methods,
constants, the default export, and the `.d.ts` beside each file.

Every example was compiled by rust-js as it is today. The JS shown is what
it writes, without its header comment and source-map line.

## Contents

- [Functions](#functions)
- [Names](#names)
- [Parameters and results](#parameters-and-results)
- [Optional parameters](#optional-parameters)
- [Structs, enums and their methods](#structs-enums-and-their-methods)
- [Constants, statics and thread-locals](#constants-statics-and-thread-locals)
- [Modules and `pub use`](#modules-and-pub-use)
- [The default export](#the-default-export)
- [TypeScript declarations](#typescript-declarations)
- [Calling Rust from a JS file in your app](#calling-rust-from-a-js-file-in-your-app)
- [What a JS caller can't do](#what-a-js-caller-cant-do)

## Functions

```rust
pub const MAX_RESULTS: u32 = 10;

pub fn first_word(text: &str) -> &str {
    text.split(' ').next().unwrap_or("")
}

pub fn greet(name: &str, greeting: Option<&str>) -> String {
    format!("{}, {}!", greeting.unwrap_or("Hello"), shout(name))
}

pub fn delete(id: u64) -> bool {
    id > 0
}

#[cfg_attr(rust_js, rust_js::name = "parseURL")]
pub fn parse_url(url: &str) -> Option<usize> {
    url.find(':')
}

fn shout(name: &str) -> String {
    name.to_uppercase()
}
```

```js
import { $find } from "@rust-js/runtime";

export const MAX_RESULTS = 10;

export function firstWord(text) {
  return text.split(" ")[0] ?? "";
}

export function greet(name, greeting) {
  return `${greeting ?? "Hello"}, ${shout(name)}!`;
}

export function delete$(id) {
  return id > 0n;
}

export function parseURL(url) {
  return $find(url, ":");
}

function shout(name) {
  return name.toUpperCase();
}
```

- A `pub fn` is an `export function`. A private `fn` is a function of its
  module, not exported.
- A JS caller imports it by its JS name:

```js
import { firstWord, greet } from "./lib.js";

firstWord("hello world"); // "hello"
greet("Ada"); // "Hello, ADA!"
greet("Ada", "Hi"); // "Hi, ADA!"
```

## Names

Every Rust name is camelCase in JS: a function, a field, a method, a
variant's field ([ADR 0573](../../decisions/0573-every-rust-name-is-camel-case-in-js.md)). `first_word` is `firstWord`, a field
`step_size` is `stepSize`.

- **A name that isn't the camelCase one** says so with
  `#[cfg_attr(rust_js, rust_js::name = "..")]`: `parse_url` as `parseURL`.
  Use it only where the name differs. One that repeats the camelCase name
  is an error that says to remove it:

  ```text
  error: rust-js: `first_word` is `firstWord` in JS without `#[rust_js::name = "firstWord"]`: remove it
  ```

- **A name JS reserves gets a `$`**: `delete` is `delete$`, `new` is
  `new$`. A method keeps its name, since it's a property: `Counter.new`.
- **A `const` is named as written**, `MAX_RESULTS`: JS's own convention
  for a constant ([ADR 0579](../../decisions/0579-a-thread-local-is-named-as-a-variable.md)).
- **A `thread_local!` is named as a module's variable**: `CLICK_COUNT` is
  `clickCount` ([ADR 0579](../../decisions/0579-a-thread-local-is-named-as-a-variable.md)).

## Parameters and results

A value crosses as the shape it has in JS. Each shape is in
[JavaScript types](types.md); these are the ones a caller meets most.

```rust
pub struct Point {
    pub x: f64,
    pub y: f64,
}

pub fn total(prices: &[f64]) -> f64 {
    prices.iter().sum()
}

pub fn pair(id: u32, name: &str) -> (u32, String) {
    (id, name.to_string())
}

pub fn shift(point: &mut Point, by: f64) {
    point.x += by;
}

pub fn bump(count: &mut u32) {
    *count += 1;
}

pub fn apply(f: impl Fn(u32) -> u32, n: u32) -> u32 {
    f(n)
}

pub fn parse_port(text: &str) -> Result<u16, String> {
    text.parse().map_err(|_| format!("not a port: {text}"))
}

pub async fn fetch_count() -> u32 {
    1
}
```

```js
import { $parseInt } from "@rust-js/runtime";

export function total(prices) {
  return prices.reduce((a, b) => a + b, -0);
}

export function pair(id, name) {
  return [id, name];
}

export function shift(point, by) {
  point.x += by;
}

export function bump(count) {
  count.value = (count.value + 1) >>> 0;
}

export function apply(f, n) {
  return f(n);
}

export function parsePort(text) {
  const result = $parseInt(text, 0, 65535);
  return result.TAG === "Err" ? { TAG: "Err", _0: `not a port: ${text}` } : result;
}

export async function fetchCount() {
  return 1;
}
```

A JS caller of these:

```js
import { bump, fetchCount, parsePort, shift } from "./lib.js";

const count = { value: 1 };
bump(count);
console.log(count.value); // 2

const point = { x: 1, y: 2 };
shift(point, 3);
console.log(point.x); // 4

const port = parsePort("8080");
if (port.TAG === "Ok") console.log(port._0); // 8080

console.log(await fetchCount()); // 1
```

| Rust | What JS gives and gets |
|---|---|
| `bool`, `u8`..`u32`, `i8`..`i32`, `usize`, `f32`, `f64` | a `number` |
| `u64`, `i64`, `u128`, `i128` | a `bigint`, `21n` |
| `&str`, `String`, `char` | a `string` |
| `&[T]`, `Vec<T>`, an array | an array |
| a tuple | an array, `[id, name]` |
| a struct | an object of its fields, `{ x, y }` |
| `Option<T>` | the value, or `undefined` for `None`; `null` is taken as `None` too |
| `Result<T, E>` | `{ TAG: "Ok", _0: v }` or `{ TAG: "Err", _0: e }` |
| `&mut` to a struct, a `Vec` | the object, changed in place |
| `&mut` to a number, a string, an `Option` | a box, `{ value }`, which the caller reads back |
| `impl Fn`, `&dyn Fn`, `fn` | a function |
| an `async fn`'s result | a `Promise` ([Promises and async](async.md)) |

- **A `Result` is a value, not a throw.** JS reads its `TAG`. A panic is
  what throws ([Errors, panics and exceptions](exceptions.md)).
- **A generic function takes its trait dictionaries after its
  arguments**, `totalArea(shapes, circleShape)`: see
  [Structs, enums and traits](objects.md#generic-code-takes-a-dictionary).

## Optional parameters

A trailing `Option` parameter is one a caller may leave out ([ADR 0443](../../decisions/0443-a-value-a-caller-may-leave-out-is-an-option.md)):

```rust
pub fn greet(name: &str, greeting: Option<&str>) -> String {
    format!("{}, {}!", greeting.unwrap_or("Hello"), shout(name))
}
#
# fn shout(name: &str) -> String {
#     name.to_uppercase()
# }
```

```ts
export function greet(name: string, greeting?: string | null | undefined): string;
```

`greet("Ada")`, `greet("Ada", undefined)` and `greet("Ada", null)` are
each `None`. An `Option` before a parameter that isn't one is declared
`T | null | undefined` and isn't optional: the caller gives it, as
`undefined` if it's `None`.

## Structs, enums and their methods

```rust
pub struct Counter {
    pub count: u32,
    pub step_size: u32,
}

impl Counter {
    pub fn new(step_size: u32) -> Counter {
        Counter {
            count: 0,
            step_size,
        }
    }

    pub fn tick(&mut self) {
        self.count += self.step_size;
    }
}

pub enum Status {
    Idle,
    Loading { percent: u8 },
    Failed(String),
}

pub fn label(status: &Status) -> String {
    match status {
        Status::Idle => "idle".to_string(),
        Status::Loading { percent } => format!("{percent}%"),
        Status::Failed(message) => message.clone(),
    }
}
```

```js
export const Counter = {
  new(stepSize) {
    return { count: 0, stepSize };
  },

  tick(counter) {
    counter.count = (counter.count + counter.stepSize) >>> 0;
  },
};

export function label(status) {
  if (status === "Idle") {
    return "idle";
  }
  if (status.TAG === "Loading") {
    return `${status.percent}%`;
  }
  return status._0;
}
```

- **A struct is a plain object.** JS makes one as a literal,
  `{ count: 0, stepSize: 2 }`, or by its constructor function.
- **A type's methods are an object named after it**, exported where one
  is `pub`. The receiver comes first: Rust's `counter.tick()` is
  `Counter.tick(counter)` ([ADR 0047](../../decisions/0047-methods.md)).
- **An enum is its variants' shapes:** a variant without fields its name,
  `"Idle"`; one with fields `{ TAG: "Loading", percent }`.

```js
import { Counter, label } from "./lib.js";

const counter = Counter.new(2);
Counter.tick(counter);
console.log(counter.count); // 2

label({ TAG: "Loading", percent: 40 }); // "40%"
label("Idle"); // "idle"
```

How traits, derives and destructors look is
[Structs, enums and traits as JavaScript sees them](objects.md).

## Constants, statics and thread-locals

```rust
use std::cell::Cell;

pub const MAX_CLICKS: u32 = 100;

pub static GREETING: &str = "hello";

thread_local! {
    pub static CLICK_COUNT: Cell<u32> = const { Cell::new(0) };
}

pub fn click() -> u32 {
    CLICK_COUNT.with(|count| {
        count.set(count.get() + 1);
        count.get()
    })
}
```

```js
export const MAX_CLICKS = 100;

export const GREETING = "hello";

export const clickCount = { value: 0 };

export function click() {
  return ((count) => {
    count.value = (count.value + 1) >>> 0;
    return count.value;
  })(clickCount);
}
```

- A `pub const` and a `pub static` are `export const`s.
- A `pub` thread-local is its module's variable, exported. A `Cell` or a
  `RefCell` in it is `{ value }`, which JS reads and Rust code sees
  change.

## Modules and `pub use`

Each module is its own file, `mod text` being `text.js` beside its
parent's ([Emitting modules](../project/modules.md)). A `pub use` of
another module's function re-exports it ([ADR 0240](../../decisions/0240-reexports.md)):

```rust
pub mod text {
    pub fn first_word(text: &str) -> &str {
        text.split(' ').next().unwrap_or("")
    }
}

pub use text::first_word;
```

```js
// src/lib.js
export { firstWord } from "./text.js";
```

```js
// src/text.js
export function firstWord(text) {
  return text.split(" ")[0] ?? "";
}
```

A `pub use` of a type re-exports nothing yet: JS imports a type's methods
from the module that defines it.

## The default export

`js::export_default!` names the function that is the module's default
export, as a Next.js page or a lazy component needs:

```rust
pub fn page() -> String {
    "home".to_string()
}

js::export_default!(page);
```

```js
export function page() {
  return "home";
}

export default page;
```

A module has one default export. A `thread_local!` can be it too, a
`memo`'s component ([Context, refs, memo](../react/api.md)).

## TypeScript declarations

`rust-js build` writes a `.d.ts` beside each module's JS that exports
something (ADRs [0196](../../decisions/0196-typescript-declarations.md), [0412](../../decisions/0412-rust-js-build-is-its-own-process.md)). TypeScript that imports the module is
checked against it. Of the first example:

```ts
export const MAX_RESULTS: number;

export function firstWord(text: string): string;

export function greet(name: string, greeting?: string | null | undefined): string;

export function delete$(id: bigint): boolean;

export function parseURL(url: string): number | null | undefined;
```

| Rust | TypeScript |
|---|---|
| `bool`, a number, a 64-bit one | `boolean`, `number`, `bigint` |
| `&str`, `String`, `char` | `string` |
| `Option<T>` | `T \| null \| undefined`; a trailing parameter `x?:`, a field `x?: T \| undefined` |
| `Vec<T>`, a slice, a tuple | `T[]`, `[A, B]` |
| a struct with named fields | an `interface` of its fields |
| an enum without fields | the union of its names, `"Red" \| "Green"` |
| an enum with fields | the union of its variants' objects, `{ TAG: "Loading"; percent: number }` |
| `impl Fn(u32) -> u32` | `(value: number) => number` |
| a type alias | `export type`, by its name |
| a function that never returns, `-> !` | `never` |
| a `pub use` of a function | `export { firstWord } from "./text.js"` |
| a doc comment | its JSDoc |

Some types aren't declared yet, and a caller of them isn't checked: a
`Result` and an `async fn`'s result are `any`, as are a tuple struct and
an `impl Iterator`, and a type's methods object, `Counter`, has its
`interface` but no declaration of its methods.

## Calling Rust from a JS file in your app

`rust-js build` writes each module's JS beside its Rust: `src/App.rs` is
`src/App.jsx`, as [the Vite tutorial](../tutorial/vite.md) has it. A JS or
TypeScript file of the app imports it by that path, as it would a JS
module of its own:

```js
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App.jsx";

createRoot(document.getElementById("root")).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
```

Where the JS goes and how a build checks it's current is
[Building the application](../project/building.md).

## What a JS caller can't do

A crate's JS is the compiler's shape of its Rust, not an API designed
for hand-written JS. A JS caller gives each value in that shape:

- **A value a function takes is Rust's, as it is in JS**: a `u64` a
  `bigint`, not a `number`; an enum `{ TAG, .. }`; a `&mut u32` a
  `{ value }`. Nothing checks it at run time: a wrong one is wrong data.
- **A generic function needs its dictionaries**, which only the crate's
  own trait impls export, `circleShape`.
- **A value with a destructor that JS holds isn't dropped** until JS
  gives it back to Rust code that drops it
  ([Drop](objects.md#drop)).
- **A library's manifest is for Rust, not JS.** A crate built as another
  crate's dependency writes `lib.manifest.json` beside its JS ([ADR 0100](../../decisions/0100-separate-crates.md)):
  what a Rust crate that uses it needs to know, which JS never reads.
