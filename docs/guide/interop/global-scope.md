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
  timers `set_timeout`, `set_interval` and their `clear_*`; a timer's delay
  is an `Option`, left out where it's `None`, as JS's is optional
  ([ADR 0443](../../decisions/0443-a-value-a-caller-may-leave-out-is-an-option.md)).
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
        js::set_timeout(task, None);
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
    setTimeout(task);
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
  A step that isn't one is refused, and so is one past a constructor,
  [a window's](#a-windows-classes): what it makes isn't read of it.
- **A std function that's `Math`'s** is asked of as `Math`'s:
  `js::is_defined(u32::leading_zeros)` is `typeof Math.clz32 !==
  "undefined"`, as React's `clz32` polyfill asks
  ([ADR 0887](../../decisions/0887-std-items-that-are-math-are-math.md),
  [Numbers](types.md#numbers)).
- **A global of ECMAScript's first edition, which every host has, isn't
  asked of** in a path through it: `Object`, `Function`, `Array`,
  `String`, `Boolean`, `Number`, `Math` and `Date`.
  `js::is_defined(js::object::prevent_extensions::<js::Unknown>)` is
  `typeof Object.preventExtensions !== "undefined"`; a later edition's,
  `Symbol`, is asked
  ([ADR 0942](../../decisions/0942-whether-a-global-is-a-function-is-is-function.md)).
- **It's a global's**: a closure, a variable, the crate's own function or
  static, a std function that isn't `Math`'s, `f64::round`, or an import
  is refused.
- **Rust is told nothing**: the binding is still there to call, and throws
  a `ReferenceError` where the host hasn't it, as JS does. Ask where the
  program may run without it.

Whether it's a function is `js::is_function(binding)`, `typeof X ===
"function"`, of what `js::is_defined` is given: a binding, a static or a
path of them. Of a path it asks the last part alone, `typeof
performance.getEntriesByType === "function"`, which throws where the host
hasn't `performance`, as JS's does; where it may not, ask that first,
`js::is_defined(performance) && js::is_function(..)`
([ADR 0942](../../decisions/0942-whether-a-global-is-a-function-is-is-function.md),
amended).
It's its own question, false of a global that's there but isn't a
function, as React asks:

```rust
pub fn can_prevent_extensions() -> bool {
    js::is_function(js::object::prevent_extensions::<js::Unknown>)
}

pub fn has_timers() -> bool {
    js::is_function(js::set_timeout)
}
```

```js
export function canPreventExtensions() {
  return typeof Object.preventExtensions === "function";
}

export function hasTimers() {
  return typeof setTimeout === "function";
}
```

A path may start from a value, its next step a method of it:
`js::is_function((selection, Selection::extend))` is `typeof
selection.extend === "function"`, as React feature-tests a selection, and
`js::is_defined` of one asks each member, never the value
([ADR 0942](../../decisions/0942-whether-a-global-is-a-function-is-is-function.md),
amended).

Of a value of a function's type that JS gives, which a JS caller may give
as anything, it asks the value: a `fn`, a `dyn Fn` lent or boxed, an `impl
Fn`, a transparent struct of one, or an `Option` of one. React's scheduler
runs a task's callback only `if (typeof callback === 'function')`. A
closure or a function the program makes is always one, and asking of it
is an error
([ADR 0942](../../decisions/0942-whether-a-global-is-a-function-is-is-function.md),
amended):

```rust
pub type Callback = Box<dyn Fn(bool) -> Option<f64>>;

pub fn run(callback: Option<Callback>, did_timeout: bool) -> Option<f64> {
    if let Some(callback) = callback
        && js::is_function(&callback)
    {
        return callback(did_timeout);
    }
    None
}
```

```js
export function run(callback, didTimeout) {
  if (typeof callback === "function") {
    return callback(didTimeout);
  }
}
```

`js::SymbolFor` itself is `Symbol.for`, asked so and called as it is where
a variable holds it; a `typeof` an earlier test of an `&&` says isn't
asked again, as React makes its selector types
([ADR 0964](../../decisions/0964-symbol-for-is-a-function-of-its-own.md)):

```rust
use js::SymbolFor;

pub fn selector_type() -> Option<SymbolFor<'static>> {
    if js::is_function(js::symbol::new) && js::is_function(SymbolFor) {
        let symbol_for = SymbolFor;
        Some(symbol_for("selector.component"))
    } else {
        None
    }
}
```

```js
export function selectorType() {
  if (typeof Symbol === "function" && typeof Symbol.for === "function") {
    const symbolFor = Symbol.for;
    return symbolFor("selector.component");
  }
}
```

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
  return createTask != null ? createTask(name) : undefined;
}

export function renderIn(task, render) {
  return task != null ? task.run(render) : render();
}
```

A method a host extends past what WebIDL declares is the method, of every
parameter it takes there, called as one, its presence asked as
`setTimeout`'s is: Chrome's `console.timeStamp(label, start, end,
trackName, trackGroup, color)`, which React's performance track logs each
phase with, is `webapi::console::time_stamp`; and `scheduler.yield(options)`,
as Chrome first shipped it and React's scheduler gives it `{signal}`, is
`scheduler.r#yield(Some(options))`
([ADR 0963](../../decisions/0963-a-hosts-extended-method-is-the-method.md),
amended):

```rust
use webapi::console;

pub fn log_render(name: &str, start: f64, end: f64) {
    if js::is_function(console::time_stamp) {
        console::time_stamp(name, Some(&start), Some(&end), Some("Components"), None, Some("primary"));
    }
}
```

```js
export function logRender(name, start, end) {
  if (typeof console.timeStamp === "function") {
    console.timeStamp(name, start, end, "Components", undefined, "primary");
  }
}
```

### A window's classes

A class's constructor is the global scope's, `error_event::new(..)`, `new
ErrorEvent(..)`. A window holds each of its own, as TypeScript's `Window &
typeof globalThis` has them: `window.new_error_event(..)` is `new
window.ErrorEvent(..)`, a `Window` method for each class a window has, of
its module's name and its constructor's parameters
([ADR 0891](../../decisions/0891-a-windows-constructor-is-the-windows.md)).
They're one class where the window is the global object, as in a page, and
two where it isn't: a frame's window, or jsdom's in a test that runs in
Node, whose global scope has no `ErrorEvent` of its own. Whether a window
has one is `js::is_defined` of its method, asked of `window`, as React's
`reportGlobalError` asks:

```rust
use webapi::{ErrorEventInit, Window, window};

pub fn report(message: &str) -> bool {
    if js::is_defined(Window::new_error_event) {
        let event = window.new_error_event(
            "error",
            Some(ErrorEventInit {
                cancelable: Some(true),
                message: Some(message),
                ..Default::default()
            }),
        );
        return window.dispatch_event(event);
    }
    true
}
```

```js
export function report(message) {
  if (typeof window !== "undefined" && typeof window.ErrorEvent !== "undefined") {
    const event = new window.ErrorEvent("error", {
      cancelable: true,
      message,
    });
    return window.dispatchEvent(event);
  }
  return true;
}
```

### A class as a value

A binding's class is a value as the crate's own is, `js::class::<T>()`
of its type: the global class TypeScript names it,
`js::class::<AbortController>()` `AbortController`, where its
`#[rust_js::types]` names that class, as webapi's each do
([ADR 0911](../../decisions/0911-a-bindings-class-is-the-global-typescript-names.md)).
A program keeps one where the host has it, and its own where it hasn't,
as React keeps `AbortController`, and makes one with `js::new`, as of
[the crate's class](export-to-javascript.md#js-classes). Its shim is the
module's own, as React's is, which only `js::class::<T>()` names: JS makes
it and reads its fields, so they and its `new` are used, to rustc's
dead-code lint and `rust-js clippy` too
([ADR 0888](../../decisions/0888-rust-js-clippy.md), amended):

```rust
use js::Class;
use webapi::AbortController;

#[cfg_attr(rust_js, rust_js::class)]
struct AbortControllerShim {
    aborted: bool,
}

#[cfg_attr(rust_js, rust_js::class)]
impl AbortControllerShim {
    fn new() -> Self {
        AbortControllerShim { aborted: false }
    }
}

thread_local! {
    // In environments without AbortController (e.g. tests)
    // replace it with a lightweight shim that only has the features we use.
    #[cfg_attr(rust_js, rust_js::name = "AbortControllerLocal")]
    static ABORT_CONTROLLER_LOCAL: &'static Class = if js::is_defined(webapi::abort_controller::new) {
        js::class::<AbortController>()
    } else {
        js::class::<AbortControllerShim>()
    };
}

pub fn controller() -> &'static AbortController {
    let controller = ABORT_CONTROLLER_LOCAL.with(|class| js::new(js::unknown(*class), ()));
    // Safety: each class makes an `AbortController`, or what React uses of one.
    unsafe { js::cast(Some(controller)) }
}
```

```js
function AbortControllerShim() {
  this.aborted = false;
}

// In environments without AbortController (e.g. tests)
// replace it with a lightweight shim that only has the features we use.
const AbortControllerLocal =
  typeof AbortController !== "undefined" ? AbortController : AbortControllerShim;

export function controller() {
  const controller = new AbortControllerLocal();
  // Safety: each class makes an `AbortController`, or what React uses of one.
  return controller;
}
```

- **What TypeScript names no class is refused**: a `JsError`, what JS
  threw, of no one class; a `JsObject` or an `Unknown`, TypeScript's
  `object` and `unknown`; a type it doesn't name, `any`.

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

What `typeof value === "object" && value !== null` takes, an object or an
array, is one value, `kind.object()`: an `Option<&JsObject>`, the value
itself, TypeScript's `object`, which keys a `WeakMap<JsObject, V>` as
neither arm's payload alone does, `Kind::Array`'s being a slice. `if let
Some(object) = js::classify(value).object()` is that `if`, its `object`
the value, and `.is_some()` of it the test
([ADR 0910](../../decisions/0910-an-object-or-an-array-is-one-value-kind-object.md)).
The guard above stays the way to read an arm's value in a `match` of
several kinds:

```rust
use js::{JsObject, Unknown, WeakMap};

thread_local! {
    #[cfg_attr(rust_js, rust_js::name = "CapturedStacks")]
    static CAPTURED_STACKS: &'static WeakMap<JsObject, String> = js::weak_map::new();
}

pub fn captured(value: Option<&'static Unknown>, stack: &str) -> Option<String> {
    if let Some(object) = js::classify(value).object() {
        if let Some(existing) = CAPTURED_STACKS.with(|captured_stacks| captured_stacks.get(object)) {
            return Some(existing);
        }
        CAPTURED_STACKS.with(|captured_stacks| captured_stacks.set(object, stack.to_string()));
    }
    None
}
```

```js
const CapturedStacks = new WeakMap();

export function captured(value, stack) {
  if (typeof value === "object" && value !== null) {
    const existing = CapturedStacks.get(value);
    if (existing != null) {
      return existing;
    }
    CapturedStacks.set(value, stack);
  }
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
  `with(|cell| ..)`: what holds the cell must see it change.
- **Exported, it's `export let`**, which another module or crate reads by
  its name, as it is now, ES's live binding. Only its module sets it:
  another's `set`, `replace`, `take` or `with_borrow_mut` of it, or its
  cell handed out, is refused, as JS lets no importer set it, so its
  module writes a `pub fn` that sets it, as React's ReactCurrentFiber
  exports `setIsRendering`. A `RefCell` of an array or an object is that
  object, which any module changes in place; its own module's `set` of it
  gives the variable the new one, `callbacks = []`, `export let callbacks`
  ([ADR 0270](../../decisions/0270-module-variables.md), amended).
- **`#[rust_js::nullable]` on one of a `Cell` or a `RefCell` of an
  `Option` makes its `None` `null`**, where it's made and where it's set,
  `export let current = null;` and `current = null;`, `Fiber | null` to
  TypeScript, as React's `let current: Fiber | null = null` is; what its
  `get`, `take` or `replace` gives is `null` or a value, given a nullable
  place as it is. On one of an `Option` itself, a `const`, its `None` is
  `null` where it's made, `export const callComponentInDEV = __DEV__ ? .. :
  null;`
  ([ADR 0586](../../decisions/0586-a-nullable-field-is-an-option.md), amended).
  A `None` a `with_borrow_mut` closure writes is `undefined`, as any a
  closure makes is. On one a generic constructor makes of an `Option`,
  `StackCursor<Option<Rc<HiddenContext>>> = create_cursor(None)`, each
  `None` the call is given for a type parameter's value is `null`,
  React's `createCursor(null)`; what it's given otherwise, and what other
  calls on the way are, as it is.
- **A `with_borrow` closure that may borrow the cell again keeps the cell**,
  borrowed while the closure runs, so a second `borrow_mut()` panics as
  std's does. What a closure, or a function given to `with_borrow`, may
  borrow is asked of its body
  ([ADR 0580](../../decisions/0580-a-call-asks-of-a-cell-only-what-it-may-borrow.md),
  [ADR 0587](../../decisions/0587-a-function-given-to-with-borrow-is-asked-as-a-closure.md)).
- **What JS runs while a borrow is held borrows what the crate's code
  does, but what its modules run as they load**: of a thread-local no
  other crate reaches, `pub` none, whose cell isn't lent, `with`'s, a
  `with_borrow` around a call into JS asks only whether another body
  borrows it mutably. One set only in `js::on_load!`, which runs once,
  before JS can call into the module, is the variable, as React's DEV
  dispatchers are, `let HooksDispatcherOnMountInDEV = null; if (__DEV__) {
  .. }`
  ([ADR 0580](../../decisions/0580-a-call-asks-of-a-cell-only-what-it-may-borrow.md),
  amended):

```rust
use std::cell::RefCell;
use std::rc::Rc;

unsafe extern "Rust" {
    safe static __DEV__: bool;
}

pub struct Dispatcher {
    pub use_id: fn() -> u32,
}

thread_local! {
    #[cfg_attr(rust_js, rust_js::name = "HooksDispatcherOnMountInDEV")]
    static ON_MOUNT_IN_DEV: RefCell<Option<Rc<Dispatcher>>> = RefCell::new(js::null());
}

js::on_load! {
    if __DEV__ {
        ON_MOUNT_IN_DEV.set(Some(Rc::new(Dispatcher { use_id: || 1 })));
    }
}

pub fn render(set_dispatcher: &dyn Fn(Option<&Dispatcher>)) {
    ON_MOUNT_IN_DEV.with_borrow(|dispatcher| set_dispatcher(dispatcher.as_deref()));
}
```

```js
let HooksDispatcherOnMountInDEV = null;

if (__DEV__) {
  HooksDispatcherOnMountInDEV = { useId: () => 1 };
}

export function render(setDispatcher) {
  setDispatcher(HooksDispatcherOnMountInDEV);
}
```

### A `static mut`

A `static mut` is its module's variable by the same rule, named as written
([ADR 0096](../../decisions/0096-statics.md), amended). Rust's own advice is
a thread-local of a `Cell`, above; a port of code that keeps one gets the
same JS:

```rust
// src/lib.rs
pub mod counter;

pub fn restart() -> usize {
    counter::set_lines(10);
    unsafe { counter::LINES }
}
```

```rust
// src/counter.rs
static mut CALLS: u32 = 0;
pub static mut LINES: usize = 0;

pub fn line() -> usize {
    unsafe {
        CALLS += 1;
        LINES += 1;
        LINES
    }
}

pub fn set_lines(lines: usize) {
    unsafe { LINES = lines }
}
```

```js
// src/lib.js
import { LINES, setLines } from "./counter.js";

export function restart() {
  setLines(10);
  return LINES;
}
```

```js
// src/counter.js
let CALLS = 0;
export let LINES = 0;

export function line() {
  CALLS = (CALLS + 1) >>> 0;
  LINES = (LINES + 1) >>> 0;
  return LINES;
}

export function setLines(lines) {
  LINES = lines;
}
```

- **A `let`, `export let` where it's `pub`**, or another module reads it,
  `const` where nothing writes it whole; its `.d.ts` is its type, `export
  let LINES: number;`.
- **Another module reads it by its name**, as it is now, ES's live
  binding, a library's consumer too. **Only its module writes it**:
  another's write of it, whole, a field or an item, or a pointer taken to
  it, `&raw mut counter::LINES`, is refused, so its module writes a `pub
  fn` that sets it, as `set_lines` does.
- **A raw pointer to it, `&raw mut COUNT` or `addr_of_mut!(COUNT)`, is a
  handle on its place**: written through where it's taken, `*addr_of_mut!
  (COUNT) += 1`, it's the variable written. Given on, `add(&raw mut COUNT)`,
  it writes the static's box, `{ value }`, which its module keeps it in; one
  it exports is refused, as a JS module importing it would read the box.

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
- **Only of Rust's closures**, which take no more than their parameters,
  or of a private function only called, by its parameters: one holding a
  JS function or another function, `queue_macrotask`, is that variable
  where nothing passes the function as a value and it's not exported, as
  React's ReactAct has `const queueSeveralMicrotasks = .. ? (callback) =>
  { .. } : queueMacrotask;`; else it stays called by its function
  ([ADR 0610](../../decisions/0610-a-forwarding-function-is-its-variable.md),
  amended):

```rust
use std::cell::Cell;

thread_local! {
    static COUNT: Cell<u32> = const { Cell::new(0) };
    static QUEUE: fn(Box<dyn Fn()>) = if js::is_defined(webapi::global::queue_microtask) {
        |callback| webapi::global::queue_microtask(callback)
    } else {
        run_now
    };
}

fn run_now(task: Box<dyn Fn()>) {
    task();
}

fn queue_task(callback: Box<dyn Fn()>) {
    QUEUE.with(|queue| queue(callback));
}

pub fn count() -> u32 {
    queue_task(Box::new(|| COUNT.set(COUNT.get() + 1)));
    COUNT.get()
}
```

```js
let COUNT = 0;
const queueTask =
  typeof queueMicrotask !== "undefined" ? (callback) => queueMicrotask(callback) : runNow;

function runNow(task) {
  task();
}

export function count() {
  queueTask(() => {
    COUNT = (COUNT + 1) >>> 0;
  });
  return COUNT;
}
```

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
  return checkTag != null ? checkTag(tag) : undefined;
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

const createTask = createTask$1 != null ? (name) => createTask$1(name) : noTask;

export function task(name) {
  return createTask(name);
}
```
