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
unsafe extern "Rust" {
    #[link_name = "./hero.png#default"]
    pub safe static HERO: &'static str;

    #[link_name = "./site.config.js#config"]
    pub safe static CONFIG: &'static Config;
}
```

```js
import HERO from "./hero.png";
import { config } from "./site.config.js";
```

The crates declare the globals every host has: `webapi::window`,
`webapi::document`, `webapi::navigator`, `webapi::history` and
`webapi::performance`.

## Global functions

A global function is a binding, as any JS function is: an ordinary function
with `#[rust_js::link_name]`. An `extern` block holds `static`s only
([ADR 0373](../../decisions/0373-one-binding-form.md)).

```rust
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

A global that only some hosts have is an `Option`. Where the host lacks it,
it's `None`; reading it doesn't throw a `ReferenceError`
([ADR 0583](../../decisions/0583-a-global-some-hosts-lack-is-an-option.md)).

```rust
use std::marker::PhantomData;

use js::JsObject;

pub struct Analytics(PhantomData<JsObject>);

impl Analytics {
    #[cfg_attr(rust_js, rust_js::link_name = "track")]
    pub fn track(&self, event: &str) {
        unreachable!()
    }
}

unsafe extern "Rust" {
    /// The analytics script's global, where the page loaded it.
    #[link_name = "analytics"]
    pub safe static ANALYTICS: Option<&'static Analytics>;
}

pub fn report(event: &str) {
    if let Some(analytics) = ANALYTICS {
        analytics.track(event);
    }
}

pub fn later(task: Box<dyn FnOnce()>) {
    match js::set_immediate {
        Some(set_immediate) => {
            set_immediate(task);
        }
        None => {
            js::set_timeout(task, 0.0);
        }
    }
}
```

```js
export function report(event) {
  const tmp = typeof analytics !== "undefined" ? analytics : undefined;
  if (tmp) {
    const analytics = tmp;
    analytics.track(event);
  }
}

export function later(task) {
  const tmp = typeof setImmediate !== "undefined" ? setImmediate : undefined;
  if (tmp) {
    const setImmediate = tmp;
    setImmediate(task);
  } else {
    setTimeout(task, 0);
  }
}
```

- **It's `typeof x !== "undefined"`**, the one JS that reads a missing
  global without throwing.
- **Each global has one declaration**: an `Option` if some host the binding
  is for lacks it, its own type if all have it. `js::set_immediate` is an
  `Option`, as browsers lack it; `webapi::performance` isn't.

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
