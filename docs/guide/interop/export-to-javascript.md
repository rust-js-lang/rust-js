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
- [Rest parameters](#rest-parameters)
- [Structs, enums and their methods](#structs-enums-and-their-methods)
- [JS classes](#js-classes)
- [Objects of methods](#objects-of-methods)
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

function delete$(id) {
  return id > 0n;
}

export function parseURL(url) {
  return $find(url, ":");
}

function shout(name) {
  return name.toUpperCase();
}

export { delete$ as delete };
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

- **A parameter's** is named so too, on the parameter, where JS's isn't
  its camelCase name, as React names a context it's given `Context`:
  `fn use_context(#[cfg_attr(rust_js, rust_js::name = "Context")] context:
  &Context)` is `function useContext(Context)`, its `.d.ts` too
  ([ADR 0855](../../decisions/0855-a-parameters-rust-js-name-is-its-js-name.md)).
  One of a pattern or of `self` is an error. So is a `let`'s, a name Rust
  can't spell: `#[cfg_attr(rust_js, rust_js::name = "$$typeof")] let
  typeof_ = ..` is `const $$typeof = ..` (ADR 0855, amended).
- **A name JS reserves gets a `$`**: `delete` is `delete$`, `new` is
  `new$`. It's exported by its own name, `export { delete$ as delete };`,
  which a JS module imports, `import { delete as remove }`
  ([ADR 0922](../../decisions/0922-an-export-is-named-first.md), amended).
  A method keeps its name, since it's a property: `Counter.new`.
- **A `const` is named as written**, `MAX_RESULTS`: JS's own convention
  for a constant ([ADR 0579](../../decisions/0579-a-thread-local-is-named-as-a-variable.md)).
- **A `thread_local!` is named as a module's variable**: `CLICK_COUNT` is
  `clickCount` ([ADR 0579](../../decisions/0579-a-thread-local-is-named-as-a-variable.md)).
  One named as another function, `const` or static of its module keeps
  its Rust name, `CACHE` beside `fn cache()`; an exported one, only
  beside another export.
- **An export's name is what its importers read, and never changes**:
  a module's `pub` items are named first, and an item it keeps to itself
  of the same name is named apart
  ([ADR 0922](../../decisions/0922-an-export-is-named-first.md)).

An exported thread-local is `clickCount` beside a private `fn
click_count()`, which is named apart, `clickCount$1`:

```rust
use std::cell::Cell;

thread_local! {
    pub static CLICK_COUNT: Cell<u32> = const { Cell::new(0) };
}

fn click_count() -> u32 {
    CLICK_COUNT.get()
}

pub fn click() -> u32 {
    CLICK_COUNT.set(click_count() + 1);
    click_count()
}
```

```js
export let clickCount = 0;

function clickCount$1() {
  return clickCount;
}

export function click() {
  clickCount = (clickCount$1() + 1) >>> 0;
  return clickCount$1();
}
```

Two exports of one name are an error that names both, as an importer
reads one export by a name:

```rust
pub fn sort_index() -> u32 {
    1
}

#[cfg_attr(rust_js, rust_js::name = "sortIndex")]
pub fn index_for_sorting() -> u32 {
    2
}
```

```
error: rust-js: `sort_index` and `index_for_sorting` are both exported as `sortIndex` in JS, where an importer reads one export by that name; give one another with `#[rust_js::name = ".."]`
 --> src/lib.rs:6:1
  |
6 | pub fn index_for_sorting() -> u32 {
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  |
note: `sortIndex` is exported here first
 --> src/lib.rs:1:1
  |
1 | pub fn sort_index() -> u32 {
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^

error: aborting due to 1 previous error
```

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
- **What a generic function copies, clones or drops by, JS leaves out**:
  a `T: Copy`'s `TCopy`, a `dropT`. A JS value is its own copy and drops
  nothing, so `createContext(defaultValue)` is how JS calls
  `create_context<T: Copy>`, a library's or a program's alike; a Rust
  caller gives one only for a value that needs it ([ADR 0289](../../decisions/0289-copy-bounds.md)).
  A `T: Clone` takes its `TClone` only where a caller gives a value that
  isn't its own clone: a number, text, a reference, an `Rc` that isn't
  counted and an `Option` of one are, so `read_context<T: Clone>` given
  them is `readContext(context)`, its clone the value. A program's `Rc` of
  a `T`'s value is counted only where a caller gives `T` a value whose
  drop runs a destructor ([ADR 0914](../../decisions/0914-a-librarys-generic-function-is-its-programs.md), amended).
  A library's takes one only where its own crate's calls give one, so its
  JS is its program's; another crate whose call needs one compiles its
  own of the function ([ADR 0914](../../decisions/0914-a-librarys-generic-function-is-its-programs.md)).

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

## Rest parameters

A function's last parameter, a slice marked
`#[cfg_attr(rust_js, rust_js::rest)]`, is JS's rest parameter: a JS
caller gives it any number of arguments, and the function is given the
slice of them ([ADR 0664](../../decisions/0664-a-functions-rest-slice-is-jss-rest-parameter.md)).
React's `formatProdErrorMessage`, which React's build calls with each
message's values:

```rust
use js::Unknown;

js::export_default! {
    fn format_prod_error_message(
        code: f64,
        #[cfg_attr(rust_js, rust_js::rest)] args: &[Option<&Unknown>],
    ) -> String {
        let mut url = format!("https://react.dev/errors/{code}");
        for (i, arg) in args.iter().enumerate() {
            url += if i == 0 { "?args[]=" } else { "&args[]=" };
            url += &js::encode_uri_component(&js::string(*arg));
        }
        format!(
            "Minified React error #{code}; visit {url} for the full message or \
             use the non-minified dev environment for full errors and additional \
             helpful warnings."
        )
    }
}

pub fn total(#[cfg_attr(rust_js, rust_js::rest)] prices: &[f64]) -> f64 {
    prices.iter().sum()
}

pub fn order(prices: &[f64]) -> f64 {
    total(&[1.5, 2.0]) + total(prices)
}
```

```js
import { $displayF64 } from "@rust-js/runtime";

export default function formatProdErrorMessage(code, ...args) {
  let url = `https://react.dev/errors/${$displayF64(code)}`;
  for (const [i, arg] of args.entries()) {
    url += i === 0 ? "?args[]=" : "&args[]=";
    url += encodeURIComponent(String(arg));
  }
  return `Minified React error #${$displayF64(code)}; visit ${url} for the full message or use the non-minified dev environment for full errors and additional helpful warnings.`;
}

export function total(...prices) {
  return prices.reduce((a, b) => a + b, -0);
}

export function order(prices) {
  return total(1.5, 2) + total(...prices);
}
```

```ts
export default function formatProdErrorMessage(code: number, ...args: (unknown | null | undefined)[]): string;

export function total(...prices: number[]): number;

export function order(prices: number[]): number;
```

- **A JS caller gives its items**, `formatProdErrorMessage(418)`,
  `formatProdErrorMessage(31, "object with keys {a}")`, `total(1, 2, 3)`.
- **A Rust caller gives a slice, as anywhere**: one held is spread,
  `total(...prices)`, and an array written out is its items,
  `total(1.5, 2)`; `&[]` is none.
- **As a value it's the arrow of its call**, `(x) => total(...x)`, which
  a caller through a `fn` pointer or an `impl Fn` gives the slice. A `fn`
  JS calls with its arguments is one of rest arguments as a value
  ([below](#a-closure-js-calls-with-its-arguments)), whose closure that
  passes them all on is the function itself.
- **It's the last parameter's, and a shared slice's, `args: &[T]`**, of
  a function or an inherent method. Anywhere else it's an error that says
  so: not the last, a `Vec<T>` or a `&mut [T]`, a pattern, a closure's,
  which takes rest arguments as a value ([below](#a-closure-js-calls-with-its-arguments)),
  a trait method's, a component's props, or a function given a
  dictionary, whose dictionaries a rest can't come before:

```rust
pub fn count(#[cfg_attr(rust_js, rust_js::rest)] items: Vec<f64>) -> usize {
    items.len()
}
```

```
error: rust-js: `#[rust_js::rest]` is a slice's, `items: &[T]`, not a `Vec<f64>`'s
 --> src/lib.rs:1:14
  |
1 | pub fn count(#[cfg_attr(rust_js, rust_js::rest)] items: Vec<f64>) -> usize {
  |              ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

error: aborting due to 1 previous error
```

### A closure JS calls with its arguments

A closure's last parameter of rest arguments as a value, `args: &dyn
js::Rest<T>`, is JS's rest parameter, `(...args) =>`, which a JS caller
gives its arguments one by one, as React's `cache` gives JS what it
caches. It reads them as the slice they deref to, of what each `T` reads
as, `Option<&Unknown>` of any value's, `&str` of `str`'s; and every call
through a type that says so spreads them, its own, a `dyn Fn`, a `Box`,
a `fn` pointer, an `impl Fn`
([ADR 0851](../../decisions/0851-a-closure-of-rest-arguments-is-jss-rest-parameter.md)):

```rust
use js::{IntoUnknown, Kind, Rest, Unknown, function};

pub fn logged(
    r#fn: &'static dyn Fn() -> Option<&'static Unknown>,
) -> impl Fn(&dyn Rest<dyn IntoUnknown>) -> Option<&'static Unknown> {
    move |args: &dyn Rest<dyn IntoUnknown>| {
        for &arg in args.iter() {
            js::set(js::unknown(&r#fn), "last", arg);
        }
        function::apply(r#fn, Some(&Kind::Null), Some(&args))
    }
}

pub fn joined() -> impl Fn(&dyn Rest<str>) -> String {
    |names: &dyn Rest<str>| names.join(", ")
}

pub fn named(names: &[&str]) -> (String, String) {
    let join = joined();
    (join(&("a", "b")), join(&names))
}
```

```js
export function logged(fn) {
  return (...args) => {
    for (let i = 0; i < args.length; i++) {
      const arg = args[i];
      fn.last = arg;
    }
    return fn.apply(null, args);
  };
}

export function joined() {
  return (...names) => names.join(", ");
}

export function named(names) {
  const join = joined();
  return [join("a", "b"), join(...names)];
}
```

```ts
export function logged(fn: () => unknown | null | undefined): (...args: (unknown | null | undefined)[]) => unknown | null | undefined;

export function joined(): (...args: string[]) => string;

export function named(names: string[]): [
    string,
    string
];
```

- **A JS caller gives its arguments**, `logged(f)(1, "a")`; a Rust one
  gives a tuple of them, `join(&("a", "b"))`, `join("a", "b")`, or a
  slice or a `Vec` held, spread, `join(...names)`, as a variadic
  binding's call is given them.
- **Its `.d.ts` is a rest parameter** of what each reads as, `(...args:
  string[]) => string`.
- **A closure that captures nothing as a `fn` of rest arguments, that
  passes them all on to a function's rest, is that function**, as
  React's renderer configs export one `shim` as each function a renderer
  lacks: `(...args) => shim(...args)` gathers all JS gives and gives all
  of it on
  ([ADR 0605](../../decisions/0605-an-arrow-that-passes-on-has-one-owner.md),
  amended):

```rust
use js::{IntoUnknown, Rest, Unknown};

fn shim(#[cfg_attr(rust_js, rust_js::rest)] _args: &[Option<&Unknown>]) -> ! {
    panic!("The current renderer does not support persistence.");
}

type Shim = fn(&dyn Rest<dyn IntoUnknown>) -> !;

#[cfg_attr(rust_js, rust_js::name = "cloneInstance")]
pub const CLONE_INSTANCE: Shim = |args| shim(args);
```

```js
function shim(...args) {
  throw new Error("The current renderer does not support persistence.");
}

export const cloneInstance = shim;
```

```ts
export const cloneInstance: (...args: (unknown | null | undefined)[]) => never;
```

- **Refused, each with what to write**: rest arguments as a value before
  a closure's last parameter; a Rust function's, whose rest is its last
  slice marked `#[rust_js::rest]`; and rest arguments given as a type
  parameter's value, `call::<&dyn Rest<T>, _>(f, args)`, whose call of an
  `F: Fn(A)` would give them as one array.

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
    return {
      count: 0,
      stepSize,
    };
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

```ts
export interface Counter {
    count: number;
    stepSize: number;
}

export const Counter: {
    "new"(stepSize: number): Counter;
    tick(counter: Counter): void;
};

export type Status = "Idle" | {
    TAG: "Loading";
    percent: number;
} | {
    TAG: "Failed";
    _0: string;
};

export function label(status: Status): string;
```

- **A struct is a plain object.** JS makes one as a literal,
  `{ count: 0, stepSize: 2 }`, or by its constructor function.
- **A type's methods are an object named after it**, exported where one
  is `pub`. The receiver comes first: Rust's `counter.tick()` is
  `Counter.tick(counter)` ([ADR 0047](../../decisions/0047-methods.md)).
  Its `.d.ts` declares the object beside the type, each `pub` method of
  its receiver first, a generic impl's type parameters on each method,
  and `new` quoted, as unquoted it'd be the type's construct signature
  ([ADR 0047](../../decisions/0047-methods.md#amended-2026-11-03-its-object-is-declared-507), amended).
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

## JS classes

A struct and its impl marked `#[cfg_attr(rust_js, rust_js::class)]` are a
JS class, written as JS wrote one before `class`: a constructor function
and its prototype ([ADR 0761](../../decisions/0761-a-rust-js-class-is-a-constructor-function-and-its-prototype.md)).
JS's `class X extends Component` runs it, `super(props)` its constructor,
and `this.setState(..)` its prototype's method. Its constructor is
exported where its struct is `pub` and its `new` or a static is; a private
one is its module's own, `function ReactTestInstance(..)`, though its
methods are `pub`. A `pub` class whose `new` isn't `pub` is its type
alone: its module's `function ReactTestInstance(..)`, and in its `.d.ts`
`declare class ReactTestInstance { .. }` and `export type {
ReactTestInstance };`
([ADR 0761](../../decisions/0761-a-rust-js-class-is-a-constructor-function-and-its-prototype.md),
amended). A class that implements a binding's class, react's `Component`,
extends it instead, an ES class, `class Surface extends Component { .. }`:
see [class components](../react/components.md#class-components) ([ADR
1041](../../decisions/1041-a-class-that-implements-a-bindings-class-extends-it.md)).
React's `Component`, and its `PureComponent`, whose prototype inherits
`Component`'s:

```rust
use js::object::assign;
use js::Unknown;

pub struct Updater {
    pub enqueue_force_update: fn(&Unknown, &str),
}

pub static NOOP: Updater = Updater {
    enqueue_force_update: |_, _| {},
};

/// Base class helpers for the updating state of a component.
#[cfg_attr(rust_js, rust_js::class)]
pub struct Component {
    pub props: Option<&'static Unknown>,
    pub updater: &'static Updater,
}

unsafe impl js::Defined for Component {}
impl js::Classed for Component {
    type Constructor = js::AnyConstructor;
}

#[cfg_attr(rust_js, rust_js::class)]
impl Component {
    pub const IS_REACT_COMPONENT: bool = true;

    pub fn new(props: Option<&'static Unknown>, updater: Option<&'static Updater>) -> Self {
        Component {
            props,
            updater: updater.unwrap_or(&NOOP),
        }
    }

    /// Forces an update.
    pub fn force_update(&self) {
        (self.updater.enqueue_force_update)(js::unknown(self), "forceUpdate");
    }
}

#[cfg_attr(rust_js, rust_js::class)]
struct ComponentDummy {}

unsafe impl js::Defined for ComponentDummy {}
impl js::Classed for ComponentDummy {
    type Constructor = js::AnyConstructor;
}

#[cfg_attr(rust_js, rust_js::class)]
impl ComponentDummy {
    fn new() -> Self {
        ComponentDummy {}
    }
}

#[cfg_attr(rust_js, rust_js::class)]
pub struct PureComponent {
    pub props: Option<&'static Unknown>,
    pub updater: &'static Updater,
}

impl js::Classed for PureComponent {
    type Constructor = js::AnyConstructor;
}

#[cfg_attr(rust_js, rust_js::class)]
impl PureComponent {
    pub fn new(props: Option<&'static Unknown>, updater: Option<&'static Updater>) -> Self {
        PureComponent {
            props,
            updater: updater.unwrap_or(&NOOP),
        }
    }
}

js::on_load! {
    js::class::<ComponentDummy>().set_prototype(js::class::<Component>().prototype());
    let pure_component_prototype = js::unknown_of(ComponentDummy::new());
    js::class::<PureComponent>().set_prototype(pure_component_prototype);
    js::set(pure_component_prototype, "constructor", js::class::<PureComponent>());
    assign(pure_component_prototype, (js::class::<Component>().prototype(),));
    js::set(pure_component_prototype, "isPureReactComponent", true);
}

pub fn mounted(props: Option<&'static Unknown>) -> Component {
    let component = Component::new(props, None);
    component.force_update();
    component
}
```

```js
export const NOOP = { enqueueForceUpdate() {} };

/** Base class helpers for the updating state of a component. */
export function Component(props, updater) {
  this.props = props;
  this.updater = updater ?? NOOP;
}

Component.prototype.isReactComponent = true;

/** Forces an update. */
Component.prototype.forceUpdate = function () {
  this.updater.enqueueForceUpdate(this, "forceUpdate");
};

function ComponentDummy() {}

export function PureComponent(props, updater) {
  this.props = props;
  this.updater = updater ?? NOOP;
}

ComponentDummy.prototype = Component.prototype;
const pureComponentPrototype = new ComponentDummy();
PureComponent.prototype = pureComponentPrototype;
pureComponentPrototype.constructor = PureComponent;
Object.assign(pureComponentPrototype, Component.prototype);
pureComponentPrototype.isPureReactComponent = true;

export function mounted(props) {
  const component = new Component(props);
  component.forceUpdate();
  return component;
}
```

```ts
export interface Updater {
    enqueueForceUpdate: (value: unknown, value2: string) => void;
}

export const NOOP: Updater;

/** Base class helpers for the updating state of a component. */
export declare class Component {
    constructor(props?: unknown | null | undefined, updater?: Updater | null | undefined);
    props?: unknown;
    updater: Updater;
    isReactComponent: boolean;
    /** Forces an update. */
    forceUpdate(): void;
}

export declare class PureComponent {
    constructor(props?: unknown | null | undefined, updater?: Updater | null | undefined);
    props?: unknown;
    updater: Updater;
}

export function mounted(props?: unknown | null | undefined): Component;
```

- **Its fields are `this`'s**, and its `fn new(..) -> Self` is the
  constructor function: the struct literal it returns is `this.props =
  props;`, each field in the order it's written, a `None`'s left out, as
  an object leaves it out. The struct's `///` doc is the constructor
  function's JSDoc, before `new`'s own.
- **A constructor of statements makes its instance in a variable**, as
  React's `FiberNode` sets fields on its flags and calls
  `Object.preventExtensions(this)`: the variable it returns, made of a
  struct literal, is `this`, its fields set where it's made, and what's
  done with it in between is done with `this`. Set again, `fiber = Fiber
  { .. }`, it's refused:

```rust
use std::cell::Cell;

use js::Unknown;

thread_local! {
    static ENABLE_PROFILER_TIMER: Cell<bool> = const { Cell::new(true) };
}

#[cfg_attr(rust_js, rust_js::class, rust_js::name = "FiberNode")]
pub struct Fiber {
    pub tag: Cell<u8>,
    pub memoized_state: Cell<Option<&'static Unknown>>,
    #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
    pub actual_duration: Cell<Option<f64>>,
    #[cfg_attr(rust_js, rust_js::name = "_debugOwner")]
    #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
    pub debug_owner: Cell<Option<&'static Unknown>>,
}

unsafe impl js::Defined for Fiber {}

#[cfg_attr(rust_js, rust_js::class)]
impl Fiber {
    pub fn new(tag: u8, dev: bool) -> Self {
        let fiber = Fiber {
            tag: Cell::new(tag),
            memoized_state: Cell::new(js::null()),
            actual_duration: Cell::new(None),
            debug_owner: Cell::new(None),
        };
        if ENABLE_PROFILER_TIMER.get() {
            fiber.actual_duration.set(Some(-0.0));
        }
        if dev {
            fiber.debug_owner.set(js::null());
            js::object::prevent_extensions(&fiber);
        }
        fiber
    }
}

pub fn create_fiber(tag: u8, dev: bool) -> Fiber {
    Fiber::new(tag, dev)
}
```

```js
const enableProfilerTimer = true;

export function FiberNode(tag, dev) {
  this.tag = tag;
  this.memoizedState = null;
  if (enableProfilerTimer) {
    this.actualDuration = -0;
  }
  if (dev) {
    this._debugOwner = null;
    Object.preventExtensions(this);
  }
}

export function createFiber(tag, dev) {
  return new FiberNode(tag, dev);
}
```

```ts
export declare class FiberNode {
    constructor(tag: number, dev: boolean);
    tag: number;
    memoizedState: unknown | null | undefined;
    actualDuration?: number | null | undefined;
    _debugOwner?: unknown | null | undefined;
}

export function createFiber(tag: number, dev: boolean): FiberNode;
```

- **A constructor may give its instance shared, `Rc<Self>`**, as React's
  Flight server's `RequestInstance` hands `this` to the task it makes:
  the `Rc::new(Request { .. })` it makes is `this`, and `Request::new(..)`
  is `new RequestInstance(..)`, the `Rc` its callers hold. An `Rc` the
  crate counts, whose counts it reads or whose last owner runs a
  destructor, is `{ value, strong, weak }`, which `new` can't give: refused,
  saying to give `Self`
  ([ADR 0761](../../decisions/0761-a-rust-js-class-is-a-constructor-function-and-its-prototype.md),
  amended, rd554):

```rust
use std::cell::RefCell;
use std::rc::Rc;

pub struct Task {
    pub request: Rc<Request>,
}

#[cfg_attr(rust_js, rust_js::class, rust_js::name = "RequestInstance")]
pub struct Request {
    pub pinged_tasks: RefCell<Vec<Rc<Task>>>,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Request {
    fn new(
        tasks: u32, // DEV-only
    ) -> Rc<Self> {
        let request = Rc::new(Request {
            pinged_tasks: RefCell::new(vec![]),
        });
        for _ in 0..tasks {
            let task = Rc::new(Task {
                request: Rc::clone(&request),
            });
            request.pinged_tasks.borrow_mut().push(task);
        }
        request
    }
}

pub fn create_request(tasks: u32) -> Rc<Request> {
    Request::new(tasks)
}
```

```js
function RequestInstance(
  tasks, // DEV-only
) {
  this.pingedTasks = [];
  for (let i = 0; i < tasks; i++) {
    const task = { request: this };
    this.pingedTasks.push(task);
  }
}

export function createRequest(tasks) {
  return new RequestInstance(tasks);
}
```

- **The struct's `#[rust_js::name]` names the class**, `FiberNode` of
  React's `Fiber`, as a binding's class is named, in its JS and its
  declaration. The class is where its impl is, its constructor function
  and its declaration both, though its struct is another module's, as
  React's `Fiber` is ReactInternalTypes' and its `FiberNode` ReactFiber's.
- **A method of `&self` or `&mut self` is the prototype's**, a `function`
  reading `this`, not an arrow, so `this` is the instance it's called on,
  a JS subclass's too. A Rust call of it is a call of it on the value,
  `component.forceUpdate()`, and `Component::new(..)` is `new
  Component(..)`. As a value, it's the arrow of its call, `(a) =>
  a.forceUpdate()`.
- **An associated `const` is the prototype's property**, camelCase as a
  method is: `IS_REACT_COMPONENT` is `isReactComponent`. A function of no
  `self` is the constructor's own, `Component.from = function (..) { ..
  }`.
- **A method marked `#[cfg_attr(rust_js, rust_js::getter)]` is the
  prototype's getter**, as JS's `get leftDiagonal() { .. }`:
  `Object.defineProperty(QueenPosition.prototype, "leftDiagonal", { get()
  { .. } })`, declared `get leftDiagonal(): number`. Its Rust call reads
  it, `position.leftDiagonal`, and as a value it's the arrow of that read.
  It's a method of `&self` alone that gives a value: one of `&mut self`,
  of arguments, of no `self`, `async`, or of a struct that's no class is
  refused.
- **Its `impl fmt::Display` is the prototype's `toString`**, as JS's
  `toString() { return `${this.rowIndex},${this.columnIndex}`; }`: Rust's
  `{p}` and `p.to_string()` are JS's text of it, `${p}`, so JS's
  `String(p)` gives Rust's text too, and a `T: Display` given it shows it
  by `String`. It's declared `toString(): string`.
- **The crate's trait each impl of which is a class's is each class's
  prototype's**, as React DOM's two roots share `render`: each method,
  the impl's or the trait's default, is a prototype method, a default
  compiled for each class with `Self` known. A method its impl gives
  only a reference to a field of `self` with, `fn counter(&self) ->
  &Cell<u32> { &self.count }`, is that field: read in place wherever
  it's called, its borrows the field's own, checked only where one may
  meet another, as an inherent method's are, and no method of its own,
  so it may be named as the field. Where two
  classes' copies print alike, one function is assigned to both, the
  later first. A Rust call is a call on the value, `root.render(..)`;
  each class declares the trait's methods. A default may take rest
  arguments, `#[rust_js::rest]`, which a class's impl then takes as it
  gives them
  ([ADR 1035](../../decisions/1035-a-trait-of-classes-is-each-classs-prototypes.md)):

```rust
use std::cell::Cell;

pub trait Counter {
    fn counter(&self) -> &Cell<u32>;

    fn step(&self, by: u32) -> u32 {
        self.counter().set(self.counter().get() + by);
        self.counter().get()
    }
}

#[cfg_attr(rust_js, rust_js::class)]
pub struct Clicks {
    pub count: Cell<u32>,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Clicks {
    pub fn new() -> Self {
        Clicks { count: Cell::new(0) }
    }
}

impl Counter for Clicks {
    fn counter(&self) -> &Cell<u32> {
        &self.count
    }
}

#[cfg_attr(rust_js, rust_js::class)]
pub struct Keys {
    pub count: Cell<u32>,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Keys {
    pub fn new() -> Self {
        Keys { count: Cell::new(0) }
    }
}

impl Counter for Keys {
    fn counter(&self) -> &Cell<u32> {
        &self.count
    }
}
```

```js
export function Clicks() {
  this.count = 0;
}

Keys.prototype.step = Clicks.prototype.step = function (by) {
  this.count = (this.count + by) >>> 0;
  return this.count;
};

export function Keys() {
  this.count = 0;
}
```

  Refused, each saying what to write: such a trait bounding a type
  parameter or a `dyn`'s, which no dictionary is made for; of type
  parameters, supertraits, or items that aren't methods of `&self` or
  `&mut self`; a method named as a field, but one that gives it; one
  that gives a field held as a function, which it's no function JS has;
  an impl outside its class's module; and an impl's own method of a
  default that takes rest arguments.
- **Its `PartialEq` of `std::ptr::eq` and `Hash` of `std::ptr::hash`**,
  equal only to itself, as React's fibers in a `Set` are, have no JS of
  their own: `a == b` is `a === b`, and a `HashSet` of them a `Set` of
  the objects ([ADR 0864](../../decisions/0864-a-type-equal-only-to-itself-keys-a-set-by-itself.md)).
- **`js::class::<T>()` is the constructor itself**, `Component`, a
  `&js::Class`: its `prototype()` and `set_prototype(..)` are JS's, which
  a prototype chain is written with, as JS writes it. Of a binding's
  type, it's the global class TypeScript names, `AbortController`
  ([A class as a value](global-scope.md#a-class-as-a-value)). A class it
  names is `js::Classed`, by one line beside the struct, `impl js::Classed
  for Component { type Constructor = js::AnyConstructor; }`, untyped, as
  the error without it says, the user's decision ([ADR
  1049](../../decisions/1049-js-class-is-typed-by-classed.md)): rustc can't
  default it, and a crate's class is held untyped, made by its `new`. A
  typed array's class is typed, `js::Class<fn(&'static ArrayBuffer, f64,
  f64) -> ArrayBufferView>`, which `js::new(class, (buffer, offset,
  length))` constructs. `js::unknown(..)` of
  an instance needs its `unsafe impl js::Defined`, a marker. A class it
  hands JS is used, its `new`, its impl's items, its fields and what they
  use, as JS makes it and reads them: a private one, which only
  `js::class::<T>()` names, needs no `#[allow(dead_code)]`, to rustc or
  `rust-js clippy` ([ADR 0888](../../decisions/0888-rust-js-clippy.md), amended).
- **Its declaration is a class's**, `export declare class Component {
  constructor(..); props?: unknown; forceUpdate(): void; }`, which a
  TypeScript `class X extends Component` type-checks against.
- **It's the one way to make a JS class.** A struct without the
  attribute is a plain object, and its impl an object of functions, as
  [above](#structs-enums-and-their-methods): the JS shape a caller sees,
  which the attribute, where Rust meets JS, says.
- **Refused, each with what to write**: a tuple or unit struct; a generic
  one; a struct literal of it outside its `new`, which would be no
  instance; a trait impl of it with items but `Display`'s, those of
  its address and a trait of classes', a `derive(Clone)` among them,
  whose methods would be a dictionary's, not the prototype's; a constructor's instance set again; a
  method of
  `self` by value; a method or constant named as a field, which
  `this.name` would hide; a class of no `new`; the prototype of a class
  with prototype methods replaced where it can't be set before them,
  [below](#a-prototype-its-load-replaces); and the attribute on one of the
  struct and its impl alone.

### A prototype its load replaces

A `js::on_load!` whose one statement replaces a class's prototype, of
globals and imports alone, is printed right after the class's constructor,
before its methods are assigned to the prototype it makes, as React's
Flight client subclasses Promise
([ADR 0761](../../decisions/0761-a-rust-js-class-is-a-constructor-function-and-its-prototype.md),
amended, #448):

```rust
use std::cell::Cell;

#[cfg_attr(rust_js, rust_js::class)]
pub struct ReactPromise {
    pub status: Cell<&'static str>,
    pub value: Cell<f64>,
}

impl js::Classed for ReactPromise {
    type Constructor = js::AnyConstructor;
}

#[cfg_attr(rust_js, rust_js::class)]
impl ReactPromise {
    pub fn new(status: &'static str, value: f64) -> Self {
        ReactPromise {
            status: Cell::new(status),
            value: Cell::new(value),
        }
    }

    pub fn then(&self, resolve: &dyn Fn(f64)) {
        if self.status.get() == "fulfilled" {
            resolve(self.value.get());
        }
    }
}

js::on_load! {
    // We subclass Promise.prototype so that we get other methods like .catch
    js::class::<ReactPromise>().set_prototype(js::object::create(
        Some(js::class::<js::Promise<()>>().prototype()),
        None,
    ));
}
```

```js
export function ReactPromise(status, value) {
  this.status = status;
  this.value = value;
}

// We subclass Promise.prototype so that we get other methods like .catch
ReactPromise.prototype = Object.create(Promise.prototype);

Object.defineProperty(ReactPromise.prototype, "then", {
  writable: true,
  enumerable: true,
  configurable: true,
  value: function (resolve) {
    if (this.status === "fulfilled") {
      resolve(this.value);
    }
  },
});
```

Each chunk is a Promise, `chunk instanceof Promise`, and its `then` is the
class's, defined on the prototype, as an assignment would make it, where an
assignment would throw of Promise.prototype frozen (SES's lockdown). Refused, saying why: one that reads what the module makes, a
function or a `const`, which may not be made yet there; one with what the
module runs as it loads between it and the class, which would see it
replaced first; a load of other statements beside it; another module's
class; an ES class, which extends a binding's; and one replaced where a
function runs, after the methods were assigned, which would drop them. A
class of no prototype methods, `ComponentDummy` above, is replaced where
its load is.

### A state flattened into a class

A class's field `#[cfg_attr(rust_js, rust_js::flatten)]` of a `RefCell` of
a discriminated union, `#[rust_js::tag = "status"]`, is the class's own
keys: its tag and each key a variant has. A borrow reads the object, a
whole write sets the new variant's tag and fields, and its
constructor takes the keys, as React's Flight client's chunk is
`new ReactPromise("pending", null, null)`. A key an earlier variant set
stays, as upstream's do; Rust reads it only through its variant. A
borrow a `let` names, `let state = &mut *chunk.state.borrow_mut();`, is
the chunk too, each whole write through it the keys, a closure's that
captures it too; a `&mut` to the
state lent to a function, `finish(&mut chunk.state.borrow_mut())`, is a
cell of it, `{ value: chunk }`, whose value is written back key by key,
so the class's other fields stay, whatever its variants hold (#478). Its
`replace`, `take`, `swap` and `get_mut` are refused, and so is the cell
lent, `&chunk.state`, or borrowed where another borrow of it may be, as
its object counts no borrows
([ADR 1044](../../decisions/1044-a-flattened-state-is-its-classs-own-keys.md)).
Its declaration, `--declarations`', has each key as the class's
property: the tag any variant's, and each other key what any variant
holds there, or `undefined` where one holds nothing, `status: "pending"
| "fulfilled"; value: number[] | null | undefined | string;`:

```rust
use std::cell::RefCell;

#[cfg_attr(rust_js, rust_js::tag = "status")]
pub enum ChunkState {
    #[cfg_attr(rust_js, rust_js::name = "pending")]
    Pending {
        #[cfg_attr(rust_js, rust_js::nullable)]
        value: Option<Vec<u32>>,
    },
    #[cfg_attr(rust_js, rust_js::name = "fulfilled")]
    Initialized { value: String },
}

#[cfg_attr(rust_js, rust_js::class)]
pub struct ReactPromise {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub state: RefCell<ChunkState>,
}

#[cfg_attr(rust_js, rust_js::class)]
impl ReactPromise {
    pub fn new(state: ChunkState) -> Self {
        ReactPromise {
            state: RefCell::new(state),
        }
    }
}

pub fn create_pending_chunk() -> ReactPromise {
    ReactPromise::new(ChunkState::Pending { value: None })
}

pub fn resolve(chunk: &ReactPromise, value: String) -> Vec<u32> {
    let listeners = match &mut *chunk.state.borrow_mut() {
        ChunkState::Pending { value } => value.take(),
        ChunkState::Initialized { .. } => None,
    };
    *chunk.state.borrow_mut() = ChunkState::Initialized { value };
    listeners.unwrap_or_default()
}

pub fn settle(chunk: &ReactPromise, value: String) -> bool {
    let state = &mut *chunk.state.borrow_mut();
    if let ChunkState::Pending { .. } = state {
        *state = ChunkState::Initialized { value };
        return true;
    }
    false
}
```

```js
export function ReactPromise(status, value) {
  this.status = status;
  this.value = value;
}

export function createPendingChunk() {
  return new ReactPromise("pending", null);
}

export function resolve(chunk, value) {
  let listeners;
  if (chunk.status === "pending") {
    const old = chunk.value;
    listeners = old;
  } else {
    listeners = undefined;
  }
  chunk.status = "fulfilled";
  chunk.value = value;
  return listeners ?? [];
}

export function settle(chunk, value) {
  if (chunk.status === "pending") {
    chunk.status = "fulfilled";
    chunk.value = value;
    return true;
  }
  return false;
}
```

### A kind flattened into its node

A field `#[cfg_attr(rust_js, rust_js::flatten)]` of an enum can hold the
enum's keys in the parent, as TypeScript's interfaces that extend one
another are one object, `interface ComputedNode extends ReactiveNode {
value; getter }`. That needs each variant of named fields to have a key
no other variant has, and at most one variant without fields. The field
can also be a type parameter that the program gives such an enum, so a
generic graph's node holds its caller's kind
([ADR 1071](../../decisions/1071-a-flattened-enum-is-told-by-its-keys.md)).

- **Its values are its keys, with no tag:** `{ value: undefined, getter }`,
  and `{}` for the variant without fields. Each key is present even where
  it holds `undefined`.
- **Each variant is tested by its own key:** `"getter" in node`. That's
  the field marked `#[cfg_attr(rust_js, rust_js::test)]`, or else the
  variant's first key that no other variant has. The variant without
  fields is none of the others.
- **A whole write deletes the other variants' keys** before it sets its
  own, so the variant changes at runtime as Rust's does.

Its declaration is the union of its variants' objects. What flattens it
is its own members together with that union, `{ flags: number } & K`:

```rust
use std::cell::Cell;

pub struct Node<K> {
    pub flags: Cell<i32>,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub kind: K,
}

pub enum Kind {
    Signal {
        current: Cell<Option<f64>>,
    },
    Computed {
        value: Cell<Option<f64>>,
        #[cfg_attr(rust_js, rust_js::test)]
        getter: fn() -> f64,
    },
    Scope,
}

pub fn computed(getter: fn() -> f64) -> Node<Kind> {
    Node {
        flags: Cell::new(0),
        kind: Kind::Computed {
            value: Cell::new(None),
            getter,
        },
    }
}

pub fn read(node: &Node<Kind>) -> Option<f64> {
    match &node.kind {
        Kind::Signal { current } => current.get(),
        Kind::Computed { value, getter } => {
            value.set(Some(getter()));
            value.get()
        }
        Kind::Scope => None,
    }
}

pub fn into_signal(node: &mut Node<Kind>) {
    node.kind = Kind::Signal {
        current: Cell::new(None),
    };
}
```

```js
export function computed(getter) {
  return {
    flags: 0,
    value: undefined,
    getter,
  };
}

export function read(node) {
  if ("current" in node) {
    return node.current;
  }
  if ("getter" in node) {
    node.value = node.getter();
    return node.value;
  }
}

export function intoSignal(node) {
  delete node.value;
  delete node.getter;
  node.current = undefined;
}
```

## Objects of methods

A struct and its impl marked `#[cfg_attr(rust_js, rust_js::object)]` are
an object literal of its fields and its methods, each reading `this`, as
React's `createScopeInstance()` returns `{ containsNode, .. }`, functions
of its module that read the scope they're called on
([ADR 0943](../../decisions/0943-an-object-of-methods-reading-this.md)):

```rust
use std::cell::Cell;

use js::Unknown;

pub struct Fiber {
    pub state_node: Option<&'static Unknown>,
    pub r#return: Option<&'static Fiber>,
}

#[cfg_attr(rust_js, rust_js::object)]
pub struct ScopeInstance {
    pub queries: Cell<u32>,
}

unsafe impl js::Defined for ScopeInstance {}

#[cfg_attr(rust_js, rust_js::object)]
impl ScopeInstance {
    /// Whether `fiber` is in this scope.
    pub fn contains_node(&self, fiber: Option<&Fiber>) -> bool {
        let mut fiber = fiber;
        while let Some(current) = fiber {
            if js::strict_equals(current.state_node, js::unknown(self)) {
                return true;
            }
            fiber = current.r#return;
        }
        false
    }

    #[cfg_attr(rust_js, rust_js::name = "DO_NOT_USE_queryFirstNode")]
    pub fn query_first_node(&self, fiber: &Fiber) -> Option<&'static Unknown> {
        self.queries.set(self.queries.get() + 1);
        fiber.state_node
    }
}

pub fn create_scope_instance() -> ScopeInstance {
    ScopeInstance {
        queries: Cell::new(0),
    }
}

pub fn contains(scope: &ScopeInstance, fiber: &Fiber) -> bool {
    scope.contains_node(Some(fiber))
}
```

```js
/** Whether `fiber` is in this scope. */
function containsNode(fiber) {
  while (fiber != null) {
    if (fiber.stateNode === this) {
      return true;
    }
    fiber = fiber.return;
  }
  return false;
}

function DO_NOT_USE_queryFirstNode(fiber) {
  this.queries = (this.queries + 1) >>> 0;
  return fiber.stateNode;
}

export function createScopeInstance() {
  return {
    queries: 0,
    containsNode,
    DO_NOT_USE_queryFirstNode,
  };
}

export function contains(scope, fiber) {
  return scope.containsNode(fiber);
}
```

```ts
export interface Fiber {
    stateNode?: unknown;
    return?: Fiber | undefined;
}

export interface ScopeInstance {
    queries: number;
    /** Whether `fiber` is in this scope. */
    containsNode(fiber?: Fiber | null | undefined): boolean;
    DO_NOT_USE_queryFirstNode(fiber: Fiber): unknown | null | undefined;
}

export function createScopeInstance(): ScopeInstance;

export function contains(scope: ScopeInstance, fiber: Fiber): boolean;
```

- **A method of `&self` or `&mut self` is a function of its impl's
  module**, a `function` whose `this` is what it's called on, its `self`
  `this`, as a class's method's is. It's exported only where another
  module names it.
- **Each literal of the struct holds them**, after its fields, by the
  method's name, camelCase or its `#[rust_js::name]`: `{ queries: 0,
  containsNode }`. A Rust call of one is the object's,
  `scope.containsNode(..)`, and as a value it's the arrow of that call.
- **Its declaration is an interface** of its fields and its `pub`
  methods, `containsNode(fiber?: Fiber | null | undefined): boolean`.
- **Its impl's other items are any impl's**, a function of no `self` its
  type's object's, and a trait's impl any type's, its methods a
  dictionary's.
- **Refused, each with what to write**: a tuple or unit struct; a
  generic one; one that's a `#[rust_js::class]` too; a method of `self`
  by value; a method named as a field; and the attribute on one of the
  struct and its impl alone.

## Module namespaces

A unit struct and its impl marked `#[cfg_attr(rust_js,
rust_js::namespace)]` are the module they're in: its `&self` methods are
the module's exported functions, and its value is the module's
namespace, `import * as M`. React's noop renderer is given the
reconciler's namespace, a Jest instance of the module, and calls through
it, `NoopRenderer.createContainer(..)`; React DOM imports the function
([ADR 1037](../../decisions/1037-a-module-namespace-is-a-unit-struct-of-its-functions.md)):

```rust
pub mod reconciler {
    #[cfg_attr(rust_js, rust_js::namespace)]
    pub struct Reconciler;

    unsafe impl js::Defined for Reconciler {}

    #[cfg_attr(rust_js, rust_js::namespace)]
    impl Reconciler {
        pub fn create_container(&self, tag: u32) -> u32 {
            self.update_container(tag) + 1
        }

        pub fn update_container(&self, n: u32) -> u32 {
            n * 2
        }
    }
}

use reconciler::Reconciler;

pub fn dom() -> u32 {
    Reconciler.create_container(3)
}

pub fn noop(renderer: &Reconciler) -> u32 {
    renderer.create_container(4)
}

pub fn namespace() -> &'static Reconciler {
    &Reconciler
}
```

```js
// src/lib.js
import { createContainer } from "./reconciler.js";
import * as Reconciler from "./reconciler.js";

export function dom() {
  return createContainer(3);
}

export function noop(renderer) {
  return renderer.createContainer(4);
}

export function namespace() {
  return Reconciler;
}
```

```js
// src/reconciler.js
export function createContainer(tag) {
  return (updateContainer(tag) + 1) >>> 0;
}

export function updateContainer(n) {
  return (n * 2) >>> 0;
}
```

- **Its methods are its module's functions**, `export function
  createContainer(tag)`, of no `self`, declared so in its `.d.ts`.
- **A call on the namespace as Rust names it**, `Reconciler.f(..)`, or
  on a method's own `self`, is the function's, `createContainer(3)`,
  imported by its name; **a call on a value** is through it,
  `renderer.createContainer(4)`, the instance JS gave. As a value, a
  method is the arrow of that call.
- **Its value is the module's namespace**, `import * as Reconciler`, a
  library's too, and its type `typeof Reconciler`, imported so: `import
  type * as Reconciler from "./reconciler.js"`. Only a value is: a
  module that only calls it by name, `let r = &Reconciler;
  r.create_container(1)` too, imports the functions it calls alone. A `use` that renames the
  struct names the import, `use reconciler::Reconciler as DefaultConfig`
  `import * as DefaultConfig`, as it names any.
- **A method that only passes its parameters on, in order, to another
  module's function is that function**, as upstream's reconciler
  re-exports the work loop's: `pub fn batched_updates<A, R>(&self, f:
  impl FnOnce(A) -> R, a: A) -> R { work_loop::batched_updates(f, a) }`
  is `export { batchedUpdates } from "./work_loop.js"`, a run of them
  from one module one statement, and its own module calls the function.
  It must take what JS gives it as the function does: its parameters'
  types the function's, its generic parameters given as they are, its
  caller's location and its bounds' dictionaries alike, and no copy its
  callers need; else it's a function of its own. A `pub use` of that
  function by the method's name, so other modules take it as a value,
  `pub use work_loop::batched_updates;`, is the same re-export, written
  once. A `pub use` of another item by that name is an error, since JS
  exports one value by a name
  ([ADR 1037](../../decisions/1037-a-module-namespace-is-a-unit-struct-of-its-functions.md), amended).
- **A call through a value is the value's function**, the module's own
  JS, a library's too, which its consumer never compiles of its own. One
  given what may look like `None`, a type parameter's `Option`, is
  refused where the copy that call needs isn't that JS, which reads
  `Some(None)` as `None`, and so is one given a value to copy or drop that
  a library's export takes nothing for: call it on the namespace as Rust
  names it. But its result that only `.flatten()` reads, of a function
  that makes its `Option` of what it's given and returns it, reading it
  nowhere, is exact, both `None`s one:
  `noop_renderer.flush_sync_from_reconciler(f).flatten()` is
  `NoopRenderer.flushSyncFromReconciler(fn)`. So is a library's method
  called by its namespace's name in another crate, each of its calls
  there flattened, react-test-renderer's
  `ReactFiberReconciler.flush_sync_from_reconciler(r#fn).flatten()`,
  `flushSyncFromReconciler(fn)`, its library's JS, and a closure that
  only passes its argument on to it, `Box::new(|r#fn| ..)`, is the
  function, `unstable_flushSync: flushSyncFromReconciler`
  ([ADR 1037](../../decisions/1037-a-module-namespace-is-a-unit-struct-of-its-functions.md), amended).
- **Refused, each with what to write**: a struct with fields or type
  parameters; one that's a `#[rust_js::class]` or `#[rust_js::object]`
  too; an impl in another module than its struct's; an item that's no
  method of `&self`; a method that reads `self` but to call another; and
  the attribute on one of the struct and its impl alone.

### A namespace that's a value

A `pub` thread-local of a struct marked `#[cfg_attr(rust_js,
rust_js::namespace)]` is its module's namespace as a value: what its
initializer makes as the module loads, whose fields are the module's
exports, as React's noop renderer's entries are `export const { .. } =
createReactNoop(..)`
([ADR 1037](../../decisions/1037-a-module-namespace-is-a-unit-struct-of-its-functions.md), amended):

```rust
pub mod noop {
    pub struct Renderer {
        pub render: Box<dyn Fn(u32) -> u32>,
        pub act: Option<fn()>,
    }

    pub fn create(scale: u32) -> Renderer {
        Renderer {
            render: Box::new(move |n| n * scale),
            act: None,
        }
    }
}

use noop::{Renderer, create};

thread_local! {
    #[cfg_attr(rust_js, rust_js::namespace)]
    pub static NOOP: Renderer = create(3);
}
```

```js
// src/lib.js
import { create } from "./noop.js";

export const { render, act } = create(3);
```

- **Each field is an export**, by its JS name, `#[rust_js::name]`'s too,
  declared in its `.d.ts`; a `None` the value leaves out is an export
  that's `undefined`, as upstream's `act` is.
- **Made as its module loads**, never where it's first used.
- **JS reads it, Rust doesn't**: a Rust read, `NOOP.with(..)`, is
  refused; so is one of no struct of named fields, or not `pub`.

An entry that exports its own list of what a function makes, in its own
order, as each of React's noop entries lists its names, is a struct of its
own, and a literal of it that takes each field, in that order, from the
same-named field of the one value its initializer makes: that value taken
apart, each export the value's field (rd610, the user's decision):

```rust
pub mod noop {
    pub struct Renderer {
        pub render: Box<dyn Fn(u32) -> u32>,
        pub act: Option<fn()>,
        pub size: u32,
    }

    pub fn create(scale: u32) -> Renderer {
        Renderer {
            render: Box::new(move |n| n * scale),
            act: None,
            size: scale,
        }
    }
}

/// What this entry exports of the renderer.
pub struct Exports {
    pub size: u32,
    pub render: Box<dyn Fn(u32) -> u32>,
}

thread_local! {
    #[cfg_attr(rust_js, rust_js::namespace)]
    pub static NOOP: Exports = {
        let renderer = noop::create(3);
        Exports {
            size: renderer.size,
            render: renderer.render,
        }
    };
}
```

```js
// src/lib.js
import * as Noop from "./noop.js";

export const { size, render } = Noop.create(3);
```

- **Only what it lists is exported**: `act` isn't one of this module's,
  `"act" in entry` is `false`.
- **Each field is taken whole, once, in its struct's order**, of the same
  JS name; any other literal, `size: renderer.size + 1`, or another order,
  is made as written, `export const { .. } = (() => { .. })()`.
- **Its struct's comments are its list's**: one on lines of its own above
  a field, `// TODO: Remove this after callers migrate.`, is above that
  export, and one after the last field before the list's `}`, as React's
  noop entries write theirs, `// TODO: ..` above `unstable_runWithPriority,`
  and `// $FlowFixMe[..]` last; a field's doc comment is its `.d.ts`
  declaration's
  ([ADR 1037](../../decisions/1037-a-module-namespace-is-a-unit-struct-of-its-functions.md), amended).

## Constants, statics and thread-locals

```rust
use std::cell::Cell;

pub const MAX_CLICKS: u32 = 100;

pub static GREETING: &str = "hello";

thread_local! {
    pub static CLICK_COUNT: Cell<u32> = const { Cell::new(0) };
}

pub fn click() -> u32 {
    CLICK_COUNT.set(CLICK_COUNT.get() + 1);
    CLICK_COUNT.get()
}
```

```js
export const MAX_CLICKS = 100;

export const GREETING = "hello";

export let clickCount = 0;

export function click() {
  clickCount = (clickCount + 1) >>> 0;
  return clickCount;
}
```

- A `pub const` and a `pub static` are `export const`s. One of a
  struct of functions, the crate's own or a library's, React's
  `Dispatcher`, is the object its initializer writes, `export const
  HOOKS_DISPATCHER = { useId, useMemo: (create) => create() }`
  ([ADR 0096](../../decisions/0096-statics.md), amended).
- A `pub` thread-local is its module's variable, exported. Of a `Cell`, or
  a `RefCell` of a number, text or an `Option`, it's what the cell holds,
  `export let`, which an importer, JS's or Rust's, reads as it is now,
  ES's live binding, and `export const` where nothing sets it. Only its
  module sets it: another module's `set`, `replace`, `take` or
  `with_borrow_mut` of it, or its cell handed out by `with`, is refused,
  so write a `pub fn` in its module that sets it, as JS does. Its own
  module's `with_borrow_mut(|depth| depth.get_or_insert_default())` sets
  it in place, `depth ??= 0`. One whose own module needs its cell, which
  it hands on, `with_borrow_mut(fill)`, or borrows in a `with`, is
  refused: read and set it with `get`, `set`, `with_borrow` and
  `with_borrow_mut`, using in their closures what they give
  ([ADR 0270](../../decisions/0270-module-variables.md), amended). A `RefCell` of
  an array, a `Map`, a `Set` or an object is that object, which any
  module changes in place
  ([ADR 0863](../../decisions/0863-a-refcell-of-an-object-is-the-object.md)).
- A `pub static mut` is its module's variable by the same rule, `export
  let`, named as written: an importer reads it as it is now, only its
  module writes it, and another module's write of it, or a pointer taken
  to it there, is refused. One its module gives a pointer to on,
  `add(&raw mut TOTAL)`, is refused, as its box is what the pointer writes
  ([ADR 0096](../../decisions/0096-statics.md), amended;
  [Module variables](global-scope.md#a-static-mut)).

A type's constants, an inherent impl's `pub const`s of a `pub` type, are
its object's, as its methods are, and each use is its value where it's
written, as TypeScript's `const enum` compiles: named flags of an `i32`
field, which a fieldless enum's variants can't be combinations of
([ADR 0410](../../decisions/0410-an-impls-function-constant-is-code.md#amendment-a-types-value-constants-are-its-objects-498),
amended):

```rust
pub struct ReactiveFlags;

impl ReactiveFlags {
    #[cfg_attr(rust_js, rust_js::name = "Mutable")]
    pub const MUTABLE: i32 = 1;
    #[cfg_attr(rust_js, rust_js::name = "Watching")]
    pub const WATCHING: i32 = 2;
}

pub fn watched(flags: i32) -> bool {
    flags & (ReactiveFlags::MUTABLE | ReactiveFlags::WATCHING) != 0
}
```

```js
export const ReactiveFlags = { Mutable: 1, Watching: 2 };

export function watched(flags) {
  return (flags & (1 | 2)) !== 0;
}
```

```ts
export const ReactiveFlags: {
    readonly Mutable: number;
    readonly Watching: number;
};

export function watched(flags: number): boolean;
```

- **A private type's, or a generic impl's, are no object**: JS can't name
  them, and each use is its value as before. One of a type with `pub`
  methods too is its object's, declared with them, its constants first
  (#507).
- **A constant whose value rustc can't hold as a value tree**, semver's
  `pub const STAR: Self = VersionReq { comparators: Vec::new() }`, has its
  initializer lowered as code, `export const VersionReq = { STAR: {
  comparators: [] } }`. Each use copies it where Rust copies (#513).

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

A `pub use` of a type has no JS: JS imports a type's methods from the
module that defines it. Its declarations re-export the type,
`export type { Kind } from "./shapes.js";`. A class's is its
constructor's, and a module's its namespace, `export * as ns from`
([ADR 0874](../../decisions/0874-a-modules-pub-uses-are-re-exported-as-written.md)).

A function that only passes its parameters on, each as it is, in order,
to a function at one instantiation is that function, re-exported by its
own name, and declared at that instantiation, as React's server entries
re-export `decodeAction` at their bundler's manifest
([ADR 1064](../../decisions/1064-a-function-that-only-passes-its-parameters-on-is-its-re-export.md)):

```rust
pub mod action {
    pub fn decode<M: ?Sized>(body: &str, _manifest: &M) -> usize {
        body.len()
    }
}

pub struct ServerManifest {
    pub id: u32,
}

pub fn decode_action(body: &str, server_manifest: &ServerManifest) -> usize {
    action::decode::<ServerManifest>(body, server_manifest)
}
```

```js
// src/lib.js
export { decode as decodeAction } from "./action.js";
```

```ts
export interface ServerManifest {
    id: number;
}

export function decodeAction(body: string, serverManifest: ServerManifest): number;
```

- **Its callee is a plain function**, the crate's other module's or a
  library's, not a method, a closure or a binding, and its parameters'
  and result's types are the callee's at what it gives it.
- **Its JS is the call alone**, of the function's own JS, which takes
  no more than it's given. One whose call needs JS of its own, a box, a
  dictionary or a drop, a `Some(..)` of what it's given, another
  statement, or a comment, stays a function; so does one its own module
  calls, its default, a generic one, and one kept out of line.
- **A JS caller gets the function itself**, `===` it, its `name` and
  `length` the function's ([semantics](../semantics.md), 45).

## The default export

`js::export_default!` names the function that is the module's default
export, as a Next.js page or a lazy component needs, exported as the
default alone, a `pub` one too
([ADR 0611](../../decisions/0611-a-default-export-is-where-its-macro-is.md)):

```rust
pub fn page() -> String {
    "home".to_string()
}

js::export_default!(page);
```

```js
function page() {
  return "home";
}

export default page;
```

A module has one default export. A `thread_local!` can be it too, a
`memo`'s component ([Context, refs, memo](../react/api.md)), and so can a
[JS class](#js-classes), JS's `export default class`: its constructor
function is the default, and a module that uses it imports the default
([ADR 0761](../../decisions/0761-a-rust-js-class-is-a-constructor-function-and-its-prototype.md)).

## TypeScript declarations

`rust-js build` writes a `.d.ts` beside each module's JS that exports
something (ADRs [0196](../../decisions/0196-typescript-declarations.md), [0412](../../decisions/0412-rust-js-build-is-its-own-process.md)). TypeScript that imports the module is
checked against it. Of the first example:

```ts
export const MAX_RESULTS: number;

export function firstWord(text: string): string;

export function greet(name: string, greeting?: string | null | undefined): string;

declare function delete$(id: bigint): boolean;

export function parseURL(url: string): number | null | undefined;

export { delete$ as delete };
```

| Rust | TypeScript |
|---|---|
| `bool`, a number, a 64-bit one | `boolean`, `number`, `bigint` |
| `&str`, `String`, `char` | `string` |
| `Option<T>` | `T \| null \| undefined`; a trailing parameter `x?:`, a field `x?: T \| undefined` |
| a `#[rust_js::rest]` slice `&[T]` | a rest parameter, `...args: T[]` |
| `Vec<T>`, a slice, a tuple | `T[]`, `[A, B]` |
| `HashMap<K, V>`, `BTreeMap<K, V>`; `HashSet<T>`, `BTreeSet<T>` | `Map<K, V>`; `Set<T>` ([ADR 0876](../../decisions/0876-a-maps-declaration-is-map.md)) |
| a struct with named fields | an `interface` of its fields, and of each `pub` method of `self` its impl binds of the JS object, `#[rust_js::link_name = "isPersistent"] pub fn is_persistent(&self) -> bool`'s `isPersistent(): boolean` ([ADR 0196](../../decisions/0196-typescript-declarations.md), amended) |
| an enum without fields | the union of its names, `"Red" \| "Green"` |
| an enum with fields | the union of its variants' objects, `{ TAG: "Loading"; percent: number }` |
| `impl Fn(u32) -> u32`, taken or given | `(value: number) => number` |
| a function of rest arguments as a value, `impl Fn(&dyn js::Rest<str>)` | `(...args: string[]) => void` |
| `Box<T>`, an `Rc<T>` whose counts nothing reads | `T` |
| its `Weak<T>` | `T \| null` ([ADR 0845](../../decisions/0845-an-rcs-declaration-is-what-it-points-at.md)) |
| a type alias | `export type`, by its name where it's used, a library's consumer's too ([ADR 0240](../../decisions/0240-reexports.md), amended) |
| a type alias whose `#[rust_js::types]` writes a type of its own, `pub type ReactServerValue = Unknown;` of `"string \| number \| ReactServerValue[]"` | `export type ReactServerValue = string \| number \| ReactServerValue[];`, by its name where it's used, a library's consumer's too ([ADR 0811](../../decisions/0811-console-create-task-may-be-absent.md), amended) |
| another module's struct or enum, or a library crate's whose declarations export it | `import type { Context } from "../react/ReactContext.js"`, by its name ([ADR 0210](../../decisions/0210-declarations-import-module-types.md), amended) |
| a function that never returns, `-> !` | `never` |
| a `pub use` of a function | `export { firstWord } from "./text.js"` |
| a `pub use` of a struct, an enum or an alias another module declares | `export type { Kind, Size as Measure } from "./shapes.js"` ([ADR 0240](../../decisions/0240-reexports.md), amended) |
| a doc comment | its JSDoc |
| a `//` comment before a type, a field or a function, past its attributes | the same comment before its declaration, as written ([ADR 0196](../../decisions/0196-typescript-declarations.md), amended) |

Some types aren't declared yet, and a caller of them isn't checked: a
`Result` and an `async fn`'s result are `any`, as are a tuple struct,
an `impl Iterator` and a counted `Rc`, and a type's methods object, `Counter`, has its
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
