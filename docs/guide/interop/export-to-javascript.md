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

- **A parameter's** is named so too, on the parameter, where JS's isn't
  its camelCase name, as React names a context it's given `Context`:
  `fn use_context(#[cfg_attr(rust_js, rust_js::name = "Context")] context:
  &Context)` is `function useContext(Context)`, its `.d.ts` too
  ([ADR 0855](../../decisions/0855-a-parameters-rust-js-name-is-its-js-name.md)).
  One of a pattern or of `self` is an error.
- **A name JS reserves gets a `$`**: `delete` is `delete$`, `new` is
  `new$`. A method keeps its name, since it's a property: `Counter.new`.
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
  a caller through a `fn` pointer or an `impl Fn` gives the slice.
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
    for (const arg of args) {
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

## JS classes

A struct and its impl marked `#[cfg_attr(rust_js, rust_js::class)]` are a
JS class, written as JS wrote one before `class`: a constructor function
and its prototype ([ADR 0761](../../decisions/0761-a-rust-js-class-is-a-constructor-function-and-its-prototype.md)).
JS's `class X extends Component` runs it, `super(props)` its constructor,
and `this.setState(..)` its prototype's method. React's `Component`, and
its `PureComponent`, whose prototype inherits `Component`'s:

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
export const NOOP = {
  enqueueForceUpdate() {},
};

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
    pub actual_duration: Cell<Option<f64>>,
    #[cfg_attr(rust_js, rust_js::name = "_debugOwner")]
    pub debug_owner: Cell<Option<&'static Unknown>>,
}

unsafe impl js::Defined for Fiber {}

#[cfg_attr(rust_js, rust_js::class)]
impl Fiber {
    pub fn new(tag: u8, dev: bool) -> Self {
        let fiber = Fiber {
            tag: Cell::new(tag),
            memoized_state: Cell::new(js::NULL),
            actual_duration: Cell::new(None),
            debug_owner: Cell::new(None),
        };
        if ENABLE_PROFILER_TIMER.get() {
            fiber.actual_duration.set(Some(-0.0));
        }
        if dev {
            fiber.debug_owner.set(js::NULL);
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
    actualDuration: number | null | undefined;
    _debugOwner: unknown | null | undefined;
}

export function createFiber(tag: number, dev: boolean): FiberNode;
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
- **Its `PartialEq` of `std::ptr::eq` and `Hash` of `std::ptr::hash`**,
  equal only to itself, as React's fibers in a `Set` are, have no JS of
  their own: `a == b` is `a === b`, and a `HashSet` of them a `Set` of
  the objects ([ADR 0864](../../decisions/0864-a-type-equal-only-to-itself-keys-a-set-by-itself.md)).
- **`js::class::<T>()` is the constructor itself**, `Component`, a
  `&js::Class`: its `prototype()` and `set_prototype(..)` are JS's, which
  a prototype chain is written with, as JS writes it. Of a binding's
  type, it's the global class TypeScript names, `AbortController`
  ([A class as a value](global-scope.md#a-class-as-a-value)). `js::unknown(..)` of
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
  instance; a trait impl of it with items but `Display`'s and those of
  its address, a `derive(Clone)` among them, whose methods would be a
  dictionary's, not the prototype's; a constructor's instance set again; a
  method of
  `self` by value; a method or constant named as a field, which
  `this.name` would hide; a class of no `new`; and the attribute on one
  of the struct and its impl alone.

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

- A `pub const` and a `pub static` are `export const`s.
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
from the module that defines it. A class's is its constructor's, and a
module's its namespace, `export * as ns from`
([ADR 0874](../../decisions/0874-a-modules-pub-uses-are-re-exported-as-written.md)).

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

export function delete$(id: bigint): boolean;

export function parseURL(url: string): number | null | undefined;
```

| Rust | TypeScript |
|---|---|
| `bool`, a number, a 64-bit one | `boolean`, `number`, `bigint` |
| `&str`, `String`, `char` | `string` |
| `Option<T>` | `T \| null \| undefined`; a trailing parameter `x?:`, a field `x?: T \| undefined` |
| a `#[rust_js::rest]` slice `&[T]` | a rest parameter, `...args: T[]` |
| `Vec<T>`, a slice, a tuple | `T[]`, `[A, B]` |
| `HashMap<K, V>`, `BTreeMap<K, V>`; `HashSet<T>`, `BTreeSet<T>` | `Map<K, V>`; `Set<T>` ([ADR 0876](../../decisions/0876-a-maps-declaration-is-map.md)) |
| a struct with named fields | an `interface` of its fields |
| an enum without fields | the union of its names, `"Red" \| "Green"` |
| an enum with fields | the union of its variants' objects, `{ TAG: "Loading"; percent: number }` |
| `impl Fn(u32) -> u32`, taken or given | `(value: number) => number` |
| a function of rest arguments as a value, `impl Fn(&dyn js::Rest<str>)` | `(...args: string[]) => void` |
| `Box<T>`, an `Rc<T>` whose counts nothing reads | `T` |
| its `Weak<T>` | `T \| null` ([ADR 0845](../../decisions/0845-an-rcs-declaration-is-what-it-points-at.md)) |
| a type alias | `export type`, by its name |
| another module's struct or enum, or a library crate's whose declarations export it | `import type { Context } from "../react/ReactContext.js"`, by its name ([ADR 0210](../../decisions/0210-declarations-import-module-types.md), amended) |
| a function that never returns, `-> !` | `never` |
| a `pub use` of a function | `export { firstWord } from "./text.js"` |
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
