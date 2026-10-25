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
`webapi::document`, `webapi::navigator`, `webapi::history`,
`webapi::performance` and `webapi::console`, the `console` namespace as a
value, whose functions are the `webapi::console` module's,
`console::log((..))` ([ADR 0667](../../decisions/0667-console-is-a-value-and-rest-arguments-are-one.md)).

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
  "undefined" && typeof Symbol.toStringTag !== "undefined"`, nothing of it
  read before ([ADR 0647](../../decisions/0647-type-name-as-react-writes-it.md)); a reference
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

A member no TypeScript lib declares, which some hosts have, is declared as
an optional property: an `Option` of its value, `None` where the host
hasn't it, as reading a missing property gives `undefined` and throws
nothing. V8's `console.createTask` is `webapi::console::create_task()`,
typed as React's Flow declares it, and the `ConsoleTask` it makes runs
what it's given, `task.run(f)`, declared as Flow's interface is, the
object type `{ run<T>(f: () => T): T }`
([ADR 0811](../../decisions/0811-console-create-task-may-be-absent.md)):

```rust
use webapi::{ConsoleTask, console};

pub fn debug_task(name: &str) -> Option<&'static ConsoleTask> {
    console::create_task().map(|create_task| create_task(name))
}

pub fn render_in(task: Option<&ConsoleTask>, render: impl FnOnce() -> u32) -> u32 {
    match task {
        Some(task) => task.run(render),
        None => render(),
    }
}
```

```js
export function debugTask(name) {
  const createTask = console.createTask;
  return createTask ? createTask(name) : undefined;
}

export function renderIn(task, render) {
  return task ? task.run(render) : render();
}
```

## What a value is: `typeof`

`js::type_of(value)` is `typeof value`, as text: for a message. To branch on
what a value is, `match` on `js::classify(value)`, whose `Kind` rustc makes
you handle in full ([ADR 0310](../../decisions/0310-any-value-as-js-reads-it.md),
[ADR 0225](../../decisions/0225-unknown-values.md)). It's a `switch (typeof
value)`: each variant its `typeof`'s case, a symbol's `case "symbol"`, an
or-pattern's stacked, `_` the `default`
([ADR 0668](../../decisions/0668-a-match-of-what-a-value-is-is-a-switch-of-its-typeof.md)).

- **`js::classify` takes an `Option<&Unknown>`**, whose `None` is
  `Kind::Undefined` or `Kind::Null`; of a value that's there,
  `classify(Some(value))`
  ([ADR 0676](../../decisions/0676-kind-tells-undefined-and-null.md)).
  `Kind::Undefined` is `case "undefined"`.
- **`Kind::Object` is `case "object"`**, an object neither `null` nor an
  array. `Kind::Object(_) | Kind::Array(_)` is one `case "object"`, and
  `Kind::Null | Kind::Object(_) | Kind::Array(_)` takes `null` too. An arm
  of `Kind::Null` alone is an `if (value === null)` first in the case of
  the arm that takes the objects, as React writes it, or before the
  `switch` where none does; `Kind::Undefined | Kind::Null` is an `if
  (value == null)` before it. Where `null` is `_`'s and an arm is
  `Kind::Object`'s, an `if` chain: `case "object"` would take `null`.
- **An arm of `Kind::Array` alone is an `if` before the `switch`**,
  `if (Array.isArray(value))`, as `typeof` says an array is an object.
  Where arrays are `_`'s and an arm is `Kind::Object`'s, it's an `if`
  chain: `case "object"` would take an array.
- **Of two cases, an `if`**, as a `match` of constants is
  ([ADR 0629](../../decisions/0629-a-match-of-constants-is-a-switch.md)).
- **An arm that reads the value binds it with a guard**, `Kind::Object(_) |
  Kind::Array(_) if let Some(value) = value`: what the arm's kind says is
  there, a guard that holds, tested by nothing, its `value` the value
  itself. And a block the arm is, `'desc: { .. }`, leaves the case where
  it's written, each `break 'desc v` a `desc = v; break;`, as a `switch`'s
  case is left early
  ([ADR 0790](../../decisions/0790-a-break-the-program-wrote-is-where-it-is-written.md)),
  and each `if` in it as written: one with no `else` has none, and an
  `||` whose branch is such a `break` is that `||`
  ([ADR 0805](../../decisions/0805-a-cases-if-is-as-written-whatever-ends-it.md)).

```rust
use js::{Kind, Unknown};

pub fn kind(value: Option<&Unknown>) -> String {
    format!("got {}", js::type_of(value))
}

pub fn width(value: Option<&Unknown>) -> f64 {
    match js::classify(value) {
        Kind::Number(n) => n,
        Kind::String(text) => js::parse_float(text),
        _ => 0.0,
    }
}

pub fn label(value: Option<&Unknown>) -> String {
    match js::classify(value) {
        Kind::Null => "null".to_string(),
        Kind::Array(items) => format!("{} items", items.len()),
        Kind::Object(object) => js::string(js::get(object, "label")),
        Kind::String(text) => text.to_string(),
        Kind::Undefined => "none".to_string(),
        _ => String::new(),
    }
}

pub fn describe(value: Option<&Unknown>) -> String {
    let desc = match js::classify(value) {
        Kind::Null => "null".to_string(),
        Kind::Object(_) | Kind::Array(_) if let Some(value) = value => 'desc: {
            if js::has(value, "name") {
                break 'desc js::string(js::get(value, "name"));
            }
            "object".to_string()
        }
        Kind::Undefined => "none".to_string(),
        _ => js::string(value),
    };
    desc + "!"
}
```

```js
export function kind(value) {
  return `got ${typeof value}`;
}

export function width(value) {
  switch (typeof value) {
    case "number":
      return value;
    case "string":
      return parseFloat(value);
    default:
      return 0;
  }
}

export function label(value) {
  if (Array.isArray(value)) {
    return `${value.length} items`;
  }
  switch (typeof value) {
    case "object":
      if (value === null) {
        return "null";
      }
      return String(value.label);
    case "string":
      return value;
    case "undefined":
      return "none";
    default:
      return "";
  }
}

export function describe(value) {
  let desc;
  switch (typeof value) {
    case "object":
      if (value === null) {
        desc = "null";
      } else {
        if ("name" in value) {
          desc = String(value.name);
          break;
        }
        desc = "object";
      }
      break;
    case "undefined":
      desc = "none";
      break;
    default:
      desc = String(value);
  }
  return desc + "!";
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
[ADR 0610](../../decisions/0610-a-forwarding-function-is-its-variable.md)).
Where each branch is one value, a function too, it's a `const` of `c ? a :
b`, as a function's `let` is: `const pick = typeof fast !== "undefined" ?
fast : (n) => ..`; a branch that does something first, as the `else` below
does, keeps the `if`
([ADR 0657](../../decisions/0657-an-if-of-one-value-each-is-a-conditional.md)):

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

One of a value where a test holds and `None` where it doesn't is assigned
only where it's something, `let componentFrameCache; if (__DEV__) {
componentFrameCache = ..; }`, as React's is
([ADR 0704](../../decisions/0704-a-module-variable-is-assigned-only-what-isnt-undefined.md));
exported, which no `let` the `if` assigns is, it's the conditional, as
`__DEV__.then_some(f)` is and React's ReactJSX writes it
([ADR 0870](../../decisions/0870-a-let-chains-value-reads-its-binding-first.md), amended):

```rust
unsafe extern "Rust" {
    pub safe static __DEV__: bool;
}

pub fn jsx_dev_checked(tag: &str) -> String {
    tag.to_string()
}

thread_local! {
    pub static JSX_DEV: Option<fn(&str) -> String> =
        if __DEV__ { Some(jsx_dev_checked) } else { None };
    static CHECK_TAG: Option<fn(&str) -> String> =
        if __DEV__ { Some(jsx_dev_checked) } else { None };
}

pub fn checked(tag: &str) -> Option<String> {
    CHECK_TAG.with(|check_tag| check_tag.map(|check| check(tag)))
}
```

```js
export function jsxDevChecked(tag) {
  return tag;
}

export const jsxDev = __DEV__ ? jsxDevChecked : undefined;
let checkTag;
if (__DEV__) {
  checkTag = jsxDevChecked;
}

export function checked(tag) {
  return checkTag ? checkTag(tag) : undefined;
}
```

A let chain of one value each, React's `if __DEV__ && let
Some(create_task) = console::create_task()`, reads what it binds first,
once, where the tests before it hold, and is then a conditional of it,
each the module's `const`; the binding's is named apart from the
thread-local's own
([ADR 0870](../../decisions/0870-a-let-chains-value-reads-its-binding-first.md)):

```rust
use webapi::{ConsoleTask, console};

const DEV: bool = true;

#[cfg_attr(rust_js, rust_js::nullable)]
fn no_task(_name: &str) -> Option<&'static ConsoleTask> {
    None
}

thread_local! {
    static CREATE_TASK: Box<dyn Fn(&str) -> Option<&'static ConsoleTask>> =
        if DEV && let Some(create_task) = console::create_task() {
            Box::new(move |name| Some(create_task(name)))
        } else {
            Box::new(no_task)
        };
}

pub fn task(name: &str) -> Option<&'static ConsoleTask> {
    CREATE_TASK.with(|create| create(name))
}
```

```js
const DEV = true;

function noTask(name) {
  return null;
}

const createTask$1 = DEV ? console.createTask : undefined;

const createTask = createTask$1 ? (name) => createTask$1(name) : noTask;

export function task(name) {
  return createTask(name);
}
```
