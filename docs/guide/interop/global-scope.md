# Access to the JavaScript global scope

JavaScript's globals are its values, `devicePixelRatio`, and its functions,
`structuredClone`. You declare each one once, and read it by its Rust name.
Your crate's own state lives in its modules, as a JS module's variables do.

## Global values

A global value is a `static` in an `unsafe extern "Rust"` block. Its
`#[link_name]` is its JS name, or a path from a global.

```rust
unsafe extern "Rust" {
    /// The `devicePixelRatio` global.
    #[link_name = "devicePixelRatio"]
    pub safe static DEVICE_PIXEL_RATIO: f64;

    /// `navigator.language`, a path from a global.
    #[link_name = "navigator.language"]
    pub safe static LANGUAGE: &'static str;

    /// What the bundler defines, `define: { __APP_VERSION__: .. }`.
    #[link_name = "__APP_VERSION__"]
    pub safe static APP_VERSION: &'static str;
}

pub fn sharp(size: f64) -> f64 {
    size * DEVICE_PIXEL_RATIO
}
```

```js
export function sharp(size) {
  return size * devicePixelRatio;
}
```

- **`safe static`** says reading it is safe, so no `unsafe` block is needed
  where it's read.
- **It's read where it's used**, `devicePixelRatio`, each time: a global may
  change.
- **The type is trusted as declared**, as a binding's is: declare an
  `Option` where the value may be `undefined` or `null`.

A `static` can be an import too, by a `#` in its `link_name`, as
[a binding's](bindings.md#where-it-comes-from) is:

```rust
# pub struct Config {
#     pub title: &'static str,
# }
#
unsafe extern "Rust" {
    #[link_name = "./hero.png#default"]
    pub safe static HERO: &'static str;

    #[link_name = "./site.config.js#config"]
    pub safe static CONFIG: &'static Config;
}

pub fn hero() -> &'static str {
    HERO
}

pub fn title() -> &'static str {
    CONFIG.title
}
```

```js
import HERO from "./hero.png";
import { config } from "./site.config.js";

export function hero() {
  return HERO;
}

export function title() {
  return config.title;
}
```

The crates declare the globals every host has: `webapi::window`,
`webapi::document`, `webapi::navigator`, `webapi::history` and
`webapi::performance`.

## Global functions

A global function is a binding, as any JS function is: an ordinary function
with `#[rust_js::link_name]`. An `extern` block holds `static`s only
([ADR 0373](../../decisions/0373-one-binding-form.md)).

```rust
# unsafe extern "Rust" {
#     #[link_name = "navigator.language"]
#     pub safe static LANGUAGE: &'static str;
#
#     #[link_name = "__APP_VERSION__"]
#     pub safe static APP_VERSION: &'static str;
# }
#
/// `structuredClone(value)`, a global function.
#[cfg_attr(rust_js, rust_js::link_name = "structuredClone")]
pub fn structured_clone(value: &str) -> String {
    unreachable!()
}

/// `globalThis.appReady = ready`.
#[cfg_attr(rust_js, rust_js::link_name = "set globalThis.appReady")]
pub fn set_app_ready(ready: bool) {
    unreachable!()
}

pub fn banner() -> String {
    set_app_ready(true);
    structured_clone(&format!("v{APP_VERSION} ({LANGUAGE})"))
}
```

```js
export function banner() {
  globalThis.appReady = true;
  return structuredClone(`v${__APP_VERSION__} (${navigator.language})`);
}
```

A global is read by its name, as JS reads it. `globalThis` is for what JS
writes on it, a global you set, `set globalThis.appReady`.

The crates bind the standard ones, called bare:

- `js`: `parse_int`, `parse_float`, `encode_uri_component`, `is_nan`, the
  timers `set_timeout`, `set_interval` and their `clear_*`.
- `webapi::global`: what every global scope has, a window's and a worker's,
  `fetch`, `queue_microtask`, `atob`, `report_error`.
- `webapi::window_global`: a window's own, `confirm`, `alert`.

## A global some hosts lack

A global is declared as TypeScript declares it, and called as JS calls it.
Whether the host has it is `js::is_defined(binding)`, `typeof X !==
"undefined"`, the one JS that asks of a missing global without throwing
([ADR 0597](../../decisions/0597-whether-the-host-has-a-global-is-is-defined.md)):

```rust
/// The analytics script's function, where the page loaded it.
#[cfg_attr(rust_js, rust_js::link_name = "gtag")]
pub fn gtag(command: &str, event: &str) {
    unreachable!()
}

pub fn report(event: &str) {
    if js::is_defined(gtag) {
        gtag("event", event);
    }
}

pub fn later(task: Box<dyn FnOnce()>) {
    if js::is_defined(js::set_immediate) {
        js::set_immediate(task);
    } else {
        js::set_timeout(task, 0.0);
    }
}
```

```js
export function report(event) {
  if (typeof gtag !== "undefined") {
    gtag("event", event);
  }
}

export function later(task) {
  if (typeof setImmediate !== "undefined") {
    setImmediate(task);
  } else {
    setTimeout(task, 0);
  }
}
```

- **It's asked of a binding**, a JS function's or a class's,
  `js::is_defined(webapi::message_channel::new)`, whose type names the
  global, or a global value's, its static given as itself,
  `js::is_defined(webapi::performance)`: a variable between asks of the
  same one. A static of a path asks each part,
  `js::is_defined(js::symbol::to_string_tag)` being `typeof Symbol !==
  "undefined" && typeof Symbol.toStringTag !== "undefined"`; a reference
  to a static, `&webapi::performance`, is refused
  ([ADR 0645](../../decisions/0645-is-defined-of-a-static-is-of-the-static.md)).
- **Each part of a path is asked**, `js::is_defined(webapi::Performance::now)`
  of the one global that type is, `typeof performance !== "undefined" &&
  typeof performance.now !== "undefined"`: a host may lack any of them.
- **A path through what a global's members give** is a tuple of
  bindings, each a method of what the one before gives, the first a
  global's: `js::is_defined((webapi::Window::document,
  webapi::Document::create_element::<webapi::tags::Div>))` is `typeof
  window !== "undefined" && typeof window.document !== "undefined" &&
  typeof window.document.createElement !== "undefined"`, as React's
  `canUseDOM` asks
  ([ADR 0624](../../decisions/0624-is-defined-of-a-path-of-bindings.md)).
  A step that isn't one is refused.
- **It's a global's**: a closure, a variable, the crate's own function or
  static, or an import is refused.
- **Rust is told nothing**: the binding is still there to call, and throws
  a `ReferenceError` where the host hasn't it, as JS does. Ask where the
  program may run without it.

## What a value is: `typeof`

`js::type_of(value)` is `typeof value`, as text: for a message. To branch on
what a value is, `match` on `js::classify(value)`, whose `Kind` rustc makes
you handle in full ([ADR 0310](../../decisions/0310-any-value-as-js-reads-it.md),
[ADR 0225](../../decisions/0225-unknown-values.md)).

```rust
use js::{Kind, Unknown};

pub fn kind(value: Option<&Unknown>) -> String {
    format!("got {}", js::type_of(value))
}

pub fn width(value: &Unknown) -> f64 {
    match js::classify(value) {
        Kind::Number(n) => n,
        Kind::String(text) => js::parse_float(text),
        _ => 0.0,
    }
}
```

```js
export function kind(value) {
  return `got ${typeof value}`;
}

export function width(value) {
  if (typeof value === "string") {
    return parseFloat(value);
  }
  if (typeof value === "number") {
    return value;
  }
  return 0;
}
```

[JavaScript types](types.md#js-values-jsobject-and-unknown) lists each
`Kind`.

## Module variables

State a module keeps, a JS module's `let`, is a `thread_local!`. JS has one
thread, so a thread-local is the module's: a cell only it reads and writes
is a variable of what the cell holds
([ADR 0270](../../decisions/0270-module-variables.md)).

```rust
use std::cell::{Cell, RefCell};

thread_local! {
    static CLICK_COUNT: Cell<u32> = const { Cell::new(0) };
    static RECENT: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

const MAX_RECENT: usize = 10;

pub fn click(label: &str) -> u32 {
    CLICK_COUNT.set(CLICK_COUNT.get() + 1);
    RECENT.with_borrow_mut(|recent| {
        if recent.len() < MAX_RECENT {
            recent.push(label.to_string());
        }
    });
    CLICK_COUNT.get()
}

pub fn last() -> Option<String> {
    RECENT.with_borrow(|recent| recent.last().cloned())
}
```

```js
let clickCount = 0;
const recent = [];

const MAX_RECENT = 10;

export function click(label) {
  clickCount = (clickCount + 1) >>> 0;
  if (recent.length < MAX_RECENT) {
    recent.push(label);
  }
  return clickCount;
}

export function last() {
  return recent.at(-1);
}
```

- **It's named as a JS variable**, camelCase: `CLICK_COUNT` is `clickCount`,
  and one holding a component, a context say, is capitalized,
  `THEME_CONTEXT` as `ThemeContext`. One named after another item of its
  module keeps its Rust name, `CACHE` beside `cache()`
  ([ADR 0579](../../decisions/0579-a-thread-local-is-named-as-a-variable.md)).
  One of a primitive, a number, text or a symbol, which nothing writes,
  keeps its name as a constant does, `OWNER_SYMBOL`
  ([ADR 0619](../../decisions/0619-a-primitive-thread-local-is-a-constant.md));
  the registry's symbol is a `const`, `js::SymbolFor`
  ([Symbols](types.md#symbols)).
- **A `let` if it's set, a `const` otherwise.**
- **A `const` is a constant, named as written**, `MAX_RECENT`.
- **It's the cell, `{ value }`,** where the cell itself is handed out,
  `with(|cell| ..)`, or another module or crate reads it: what holds the
  cell must see it change.
- **A `with_borrow` closure that may borrow the cell again keeps the cell**,
  borrowed while the closure runs, so a second `borrow_mut()` panics as
  std's does. What a closure, or a function given to `with_borrow`, may
  borrow is asked of its body
  ([ADR 0580](../../decisions/0580-a-call-asks-of-a-cell-only-what-it-may-borrow.md),
  [ADR 0587](../../decisions/0587-a-function-given-to-with-borrow-is-asked-as-a-closure.md)).

### One chosen when the module loads

A thread-local an `if` makes is a `let` each branch assigns, as JS picks
what it uses where it loads; and a function that only calls a thread-local
of a closure is that variable, by the function's name
([ADR 0601](../../decisions/0601-a-module-variable-an-if-makes-is-a-let.md),
[ADR 0610](../../decisions/0610-a-forwarding-function-is-its-variable.md)):

```rust
thread_local! {
    static GET_TIME: Box<dyn Fn() -> f64> = if js::is_defined(webapi::Performance::now) {
        let local_performance = webapi::performance;
        Box::new(move || local_performance.now())
    } else {
        let start = js::date::now();
        Box::new(move || js::date::now() - start)
    };
}

pub fn get_time() -> f64 {
    GET_TIME.with(|get_time| get_time())
}
```

```js
let getTime;
if (typeof performance !== "undefined" && typeof performance.now !== "undefined") {
  const localPerformance = performance;
  getTime = () => localPerformance.now();
} else {
  const start = Date.now();
  getTime = () => Date.now() - start;
}

export { getTime };
```

- **Not where the module's load calls the function**: a function
  declaration is hoisted, so it stays one.
- **Only of Rust's closures**, which take no more than their parameters.
  A thread-local holding a JS function stays called by its function.
