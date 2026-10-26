# Write bindings for JavaScript APIs

A binding is a Rust declaration of something JavaScript has: a function, a
class, a method, a property. You call it as Rust, rustc checks the call
against its types, and rust-js writes the JavaScript it names. Scala.js calls
these facade types; here they're ordinary Rust functions and structs.

Before you write one, check the crates rust-js ships: the `js` crate binds
the language's globals, `webapi` the browser's, `node` Node's and `react`
React's ([the crates rust-js ships](../libraries/README.md)). Write your own
for an npm package, or for your own JS files.

## Bind a function

```rust
#[cfg_attr(rust_js, rust_js::link_name = "sonner#toast")]
pub fn toast(message: &str) {
    unreachable!()
}

pub fn greet() {
    toast("Saved");
}
```

```js
import { toast } from "sonner";

export function greet() {
  toast("Saved");
}
```

- **The function's signature is the JS function's**, in TypeScript's order.
- **Its body is `unreachable!()`**: it never runs, JS's function does. rust-js
  allows its unread parameters, so you write no `#[allow]`
  ([ADR 0440](../../decisions/0440-a-binding-needs-no-allow.md)).
- **`#[rust_js::link_name]` says what JS it is**, written as
  `#[cfg_attr(rust_js, ..)]` so a plain `cargo check` reads the crate too.
- **A binding is trusted as declared.** rustc checks your calls against it;
  nothing checks it against the JS. Declare what JS really gives: an
  `Option` where it may give `undefined` or `null`, a `Result` where it may
  throw.

A JS function is always bound this way. One declared in an `extern` block is
refused, with the form to write
([ADR 0373](../../decisions/0373-one-binding-form.md)):

```
rust-js: a JS function is bound as an ordinary function, `#[cfg_attr(rust_js, rust_js::link_name = "structuredClone")] fn structured_clone(..) { unreachable!() }`, not in an `extern` block (ADR 0373)
```

An `extern` block holds a global value, a `static`: see
[Access to the JavaScript global scope](global-scope.md).

### A binding as a value

A binding taken as a `fn` is the JS function itself, read where it's taken,
as a Rust `fn` is fixed once taken. Handed to a JS callback itself,
`.map(encode)`, it's an arrow of its own parameters, so JS gives it no
more than Rust does, no index
([ADR 0602](../../decisions/0602-a-binding-taken-as-a-fn-is-its-function.md)):

```rust
#[cfg_attr(rust_js, rust_js::link_name = "encodeURIComponent")]
pub fn encode(text: &str) -> String {
    unreachable!()
}

thread_local! {
    // Captured, in case a polyfill overrides it.
    static LOCAL_ENCODE: fn(&str) -> String = encode;
}

pub fn encoded(texts: Vec<&str>) -> Vec<String> {
    texts.into_iter().map(encode).collect()
}

pub fn first(text: &str) -> String {
    LOCAL_ENCODE.with(|encode| encode(text))
}
```

```js
// Captured, in case a polyfill overrides it.
const localEncode = encodeURIComponent;

export function encoded(texts) {
  return texts.map((text) => encodeURIComponent(text));
}

export function first(text) {
  return localEncode(text);
}
```

A binding of TypeScript's `any`, `reportError(e: any)`, is generic, so
its value is of the type it's given: a `fn` of `Option<&'static
Unknown>`, a JS value of any shape, which lives as long as JS keeps it, as
a binding gives one. A `fn` of `Option<&Unknown>` of any lifetime is no
generic's: rustc says "one type is more general than the other"
([ADR 0655](../../decisions/0655-an-any-binding-as-a-value-is-a-fn-of-a-static-unknown.md)):

```rust
use js::Unknown;

thread_local! {
    static REPORT_GLOBAL_ERROR: fn(Option<&'static Unknown>) =
        if js::is_defined(webapi::global::report_error::<Option<&Unknown>>) {
            webapi::global::report_error
        } else {
            |error| webapi::console::error((error,))
        };
}

pub fn report(error: Option<&'static Unknown>) {
    REPORT_GLOBAL_ERROR.with(|report| report(error));
}
```

```js
const reportGlobalError =
  typeof reportError !== "undefined" ? reportError : (error) => console.error(error);

export function report(error) {
  reportGlobalError(error);
}
```

A variadic binding, of a TypeScript rest parameter, is a `fn` of rest
arguments as a value, `&'static dyn js::Rest<T>`, so it's the function
itself, and a call of it spreads them, as a variadic binding's call does.
A closure that takes them last is JS's rest parameter, `(...args) =>`,
and so is called spread, through a `dyn Fn` of them too
([Rest parameters](export-to-javascript.md#rest-parameters)); a Rust
function that takes them is refused: its rest is its last slice marked
`#[rust_js::rest]`
([ADR 0667](../../decisions/0667-console-is-a-value-and-rest-arguments-are-one.md),
[0851](../../decisions/0851-a-closure-of-rest-arguments-is-jss-rest-parameter.md)).
`console` itself is `webapi::console`:

```rust
use std::cell::Cell;

use js::{IntoUnknown, PropertyDescriptor, Rest, object};

type ConsoleFn = fn(&'static dyn Rest<dyn IntoUnknown>);

thread_local! {
    static SAVED_LOG: Cell<Option<ConsoleFn>> = const { Cell::new(None) };
}

fn quiet() {}

pub fn silence() {
    SAVED_LOG.set(Some(webapi::console::log));
    let quieted = PropertyDescriptor {
        value: Some(&quiet),
        ..Default::default()
    };
    object::define_property(webapi::console, "log", &quieted);
}

pub fn restore() {
    let saved = SAVED_LOG.get();
    let restored = PropertyDescriptor {
        value: Some(&saved),
        ..Default::default()
    };
    object::define_property(webapi::console, "log", &restored);
    if let Some(log) = saved {
        log(&("restored", 1));
    }
}
```

```js
let savedLog;

function quiet() {}

export function silence() {
  savedLog = console.log;
  const quieted = {
    value: quiet,
  };
  Object.defineProperty(console, "log", quieted);
}

export function restore() {
  const saved = savedLog;
  const restored = {
    value: saved,
  };
  Object.defineProperty(console, "log", restored);
  if (saved) {
    saved("restored", 1);
  }
}
```

## Where it comes from

What's before a `#` is the module, written into the `import` as it is.
What's after is the path in it: its first name is the export, and the rest
are properties ([ADR 0028](../../decisions/0028-js-module-imports.md)).

| `link_name` | JS |
|---|---|
| `"parseInt"`, `"console.log"` | a global, or a path from one |
| `"sonner#toast"` | `import { toast } from "sonner"` |
| `"node:path#join"` | `import { join } from "node:path"` |
| `"canvas-confetti#default"` | `import confetti from "canvas-confetti"`, the default export |
| `"./greet.js#*.polite"` | `import * as greet from "./greet.js"`, then `greet.polite` |
| `"chart.js#Chart.register"` | `import { Chart } from "chart.js"`, then `Chart.register` |

A relative module, `./greet.js`, is relative to the crate root's JS file;
each module that imports it gets the path from its own directory. The
build reads it, and the files of the program's own it imports, for which
of the crate's modules it loads: a module whose load reads another's
`const` in a cycle through it reads its value
([ADR 0031](../../decisions/0031-consts.md), amended). A change to one
rebuilds the crate.

```rust
#[cfg_attr(rust_js, rust_js::link_name = "canvas-confetti#default")]
pub fn confetti() {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "./greet.js#*.polite")]
pub fn polite(name: &str) -> String {
    unreachable!()
}

pub fn celebrate() {
    confetti();
    println!("{}", polite("Ada"));
}
```

```js
import confetti from "canvas-confetti";
import * as greet from "./greet.js";

export function celebrate() {
  confetti();
  console.log(`${greet.polite("Ada")}`);
}
```

- **A namespace is named by what it's imported from**, `greet`, where its
  bindings are in a file of other code too; a module of no code of its own,
  its bindings and the props they take, `mod ContextMenu`, names it,
  `import * as ContextMenu`
  ([ADR 0256](../../decisions/0256-namespace-import-names.md), amended).
- **`js::import!(f)` loads a binding's module when it's asked for**:
  `js::import!(highlight).await` is `const { highlight } = await
  import("./highlight.js")`, and nothing imports it statically
  ([ADR 0304](../../decisions/0304-dynamic-import.md)).
- **`#[rust_js::require]` reads it through `require(module)`**, where a
  module must be read that way, an ESLint plugin's
  ([ADR 0305](../../decisions/0305-require.md)). One a bundler mustn't
  see, Node's own `timers` past fake ones, is read off Node's `module`,
  `node::module`, by a name the program makes, `js::get(js::unknown(node::module),
  &require_string)`, `module[requireString]`, and called with
  `js::function::call(node_require, Some(&node::module), ("timers",))`,
  `nodeRequire.call(module, "timers")`; in an ES module, which has no
  `module`, the read throws, which `catch_unwind` catches, a `try`
  ([ADR 0305, amended](../../decisions/0305-require.md#amendment-a-require-no-bundler-sees)).
- **Named through a module that holds it, it's read through that
  module**: where `assign.rs` holds `Object.assign` as its default, a
  call of `crate::assign::assign` imports it, `import assign from
  "./assign.js"`, and is `assign(target, source)`, a static's read and a
  `fn` taken of one too; a call of `js::object::assign`, the binding
  itself, is `Object.assign(target, source)`
  ([ADR 0633](../../decisions/0633-a-binding-named-through-a-module-is-read-through-it.md),
  and [the default export](../project/modules.md#the-default-export)).

To publish bindings of an npm package for others, see
[Dependencies](../project/dependencies.md).

## Declare a JS type

A JS class or object type is a struct of one `PhantomData<JsObject>`. You
never have one by value; a binding gives you a `&'static` reference
([ADR 0111](../../decisions/0111-js-types-as-structs.md)). Its constructor,
statics and methods are bindings.

```rust
use std::marker::PhantomData;

use js::JsObject;

/// The `Chart` class of the `chart.js` package.
#[cfg_attr(rust_js, rust_js::types = "chart.js#Chart")]
pub struct Chart(PhantomData<JsObject>);

/// `Chart`'s constructor and statics.
pub mod chart {
    use super::Chart;

    /// `new Chart(canvasId)`.
    #[cfg_attr(rust_js, rust_js::link_name = "new chart.js#Chart")]
    pub fn new(canvas_id: &str) -> &'static Chart {
        unreachable!()
    }

    /// `Chart.version`, a static property.
    #[cfg_attr(rust_js, rust_js::link_name = "get chart.js#Chart.version")]
    pub fn version() -> String {
        unreachable!()
    }

    /// `Chart.register(plugin)`, a static method.
    #[cfg_attr(rust_js, rust_js::link_name = "chart.js#Chart.register")]
    pub fn register(plugin: &str) {
        unreachable!()
    }
}

impl Chart {
    /// `chart.update(mode?)`.
    #[cfg_attr(rust_js, rust_js::link_name = "update")]
    pub fn update(&self, mode: Option<&str>) {
        unreachable!()
    }

    /// `chart.width`.
    #[cfg_attr(rust_js, rust_js::link_name = "get width")]
    pub fn width(&self) -> f64 {
        unreachable!()
    }

    /// `chart.width = width`.
    #[cfg_attr(rust_js, rust_js::link_name = "set width")]
    pub fn set_width(&self, width: f64) {
        unreachable!()
    }
}

pub fn draw() -> f64 {
    chart::register("legend");
    let chart = chart::new("sales");
    chart.update(None);
    chart.update(Some("none"));
    chart.set_width(400.0);
    println!("{}", chart::version());
    chart.width()
}
```

```js
import { Chart } from "chart.js";

export function draw() {
  Chart.register("legend");
  const chart = new Chart("sales");
  chart.update();
  chart.update("none");
  chart.width = 400;
  console.log(`${Chart.version}`);
  return chart.width;
}
```

This is how `webapi` lays out each interface: the type, its members in an
`impl`, and its constructors and statics in a module of its name,
`chart::new`.

- **A method takes `&self`.** Its `link_name` is the member's name, with no
  module: it's read of the value.
- **`#[rust_js::types]`** names the type to TypeScript, in the `.d.ts`
  rust-js writes. Without it, the type is `any` there. Its value is a
  TypeScript type, which rust-js reads into the declaration it prints
  ([ADR 0811](../../decisions/0811-console-create-task-may-be-absent.md)):
  a name, of a module, `chart.js#Chart`, which the `.d.ts` imports, or of
  a namespace, `NodeJS.TypedArray`, with its arguments, `Map<string, T>`;
  a keyword, `string`; a string, `"auto"`; a union, `string | URL`; an
  array, `T[]` or `readonly T[]`; a function, `(f: () => T) => T`; or an
  object, `{ expire?: number; run<T>(f: () => T): T; [key: string]: T }`.
  A name that's one of the Rust type's type parameters, `T`, is the type
  it's given, but in a method or function of a `T` of its own. A name
  alone, `chart.js#Chart`, takes the Rust type's arguments, and `Name<>`
  none. A value it doesn't read, `A & B` or `import("m").T`, is a compile
  error that says what it reads: write `m#T`.

### A method on another crate's type

Rust allows no `impl` of a type from another crate. Bind the method as a
function whose first parameter is named `this`:

```rust
use webapi::Element;

/// Chrome's `element.scrollIntoViewIfNeeded(center?)`.
#[cfg_attr(rust_js, rust_js::link_name = "scrollIntoViewIfNeeded")]
pub fn scroll_into_view_if_needed(this: &Element, center: Option<bool>) {
    unreachable!()
}

pub fn reveal(el: &Element) {
    scroll_into_view_if_needed(el, None);
}
```

```js
export function reveal(el) {
  el.scrollIntoViewIfNeeded();
}
```

### Inheritance is `Deref`

A class that extends another derefs to it. Rust's deref coercion then gives
the subclass the parent's methods, and lets it go where the parent goes;
the JS is the object itself.

```rust
use std::marker::PhantomData;
use std::ops::Deref;

use js::JsObject;

pub struct Shape(PhantomData<JsObject>);

/// `class Circle extends Shape`.
pub struct Circle(PhantomData<JsObject>);

impl Deref for Circle {
    type Target = Shape;

    fn deref(&self) -> &Shape {
        unsafe { &*(self as *const Self).cast::<Shape>() }
    }
}

impl Shape {
    #[cfg_attr(rust_js, rust_js::link_name = "area")]
    pub fn area(&self) -> f64 {
        unreachable!()
    }
}

#[cfg_attr(rust_js, rust_js::link_name = "new shapes#Circle")]
pub fn circle(radius: f64) -> &'static Circle {
    unreachable!()
}
```

`circle(2.0).area()` is `new Circle(2).area()`. The `deref` body is no JS.

## The forms of `link_name`

| `link_name` | JS | of |
|---|---|---|
| `"f"`, `"a.b.f"` | `f(..)`, `a.b.f(..)` | a function |
| `"f"`, with `&self` or `this` | `this.f(..)` | a method |
| `"get x"` | `this.x`, or `X.x` without `this` | a getter, or a static property |
| `"set x"` | `this.x = v`, or `a.x = v` of a global | a setter |
| `"new X"` | `new X(..)`, or `new this.X(..)` with `&self` or `this` | a constructor, or one its receiver holds, a window's ([ADR 0891](../../decisions/0891-a-windows-constructor-is-the-windows.md)) |
| `"this"` | `this` | a cast, unchecked |
| `"instanceof X"` | `this instanceof X` | a cast's test |
| `"instanceof"` | `this instanceof constructor`, of a constructor given | `js::instance_of` ([ADR 0878](../../decisions/0878-instanceof-of-a-constructor-value.md)) |
| `"this()"` | `this(..)` | calling a JS function value |
| `"new this()"` | `new this(..)` | `js::new` |
| `"delete []"` | `delete this[key]` | `js::delete` |
| `"for in"` | the keys `for (const key in this)` visits | `js::keys_in` |
| `"typeof"` | `typeof this` | `js::type_of` |
| `"!!"` | `!!this` | `js::truthy` |
| `"\"\" +"` | `"" + this` | `js::concat_text` |
| `"get []"`, `"set []"`, `"in []"` | `this[key]`, `this[key] = v`, `key in this` | `js::get`, `js::set`, `js::has` |
| `"{}"`, `"{label, value}"` | `{}`, `{ label, value }` | an object of the arguments |
| `"<tag>"`, `"prop name"` | JSX | React's element bindings |

A module's `#` goes with the forms that aren't of a value: a function, `new`,
`get` of a static. A method of a module, `this` and `"pkg#f"`, is an error.

A keyed read whose result is an `Option` of what may be `None` itself, an
`Option`, a `()` or `js::Kind`, tells a key whose value is `undefined` or
`null` from a key that isn't there, `Some(None)` from `None`, as JS's read
alone can't: a method `"get"`, a `Map`'s, by `this.has(key)`,
`$someGet(this, key)`; `"get []"`, a `Dict`'s, by its own key,
`$dictGet(this, key)` ([ADR 0884](../../decisions/0884-a-maps-value-is-any-value.md)).

A getter runs once for each call of its binding, as Rust calls it: a
property an `if let` chain tests and binds, `let Some(Kind::String(url)) =
js::get(request, "url").map(js::classify)`, is read once, `const url =
request.url;`, and tested as that, not read again in the test
([ADR 0672](../../decisions/0672-a-property-a-test-reads-is-read-once.md)).

`js::concat_text` is `"" + value`: of text or a number, its `String`; of a
value of any shape, a `Result`, as `+` throws of a symbol and of an object
whose `valueOf` throws (ADR 0639). A `match` of it, or its `is_err()`, is
JS's `try`. Text joined to it is `+`, `"require".to_owned() +
&js::concat_text(n)` is `"require" + n`. JS's `a && b` of a value of any
shape is `if js::truthy(a) { b } else { a }`, and `a || b` its other
way round, `if js::truthy(a) { a } else { b }`
([ADR 0880](../../decisions/0880-a-test-of-truth-and-its-value-is-and.md)):

```rust
use js::Unknown;

pub fn draw(meta: &Unknown) -> bool {
    if js::has(meta, "scale") {
        js::set(meta, "drawn", true);
    }
    let scale = js::get(meta, "scale");
    js::truthy(scale)
}

pub fn width(n: f64) -> String {
    js::concat_text(n) + "px"
}

pub fn will_coercion_throw(value: Option<&Unknown>) -> bool {
    js::concat_text(value).is_err()
}

pub fn require_string() -> String {
    "require".to_owned() + &js::concat_text(js::math::random())
}

pub fn node_require(module: Option<&'static Unknown>, key: &str) -> Option<&'static Unknown> {
    if js::truthy(module) {
        js::get(module, key)
    } else {
        module
    }
}
```

```js
export function draw(meta) {
  if ("scale" in meta) {
    meta.drawn = true;
  }
  const scale = meta.scale;
  return !!scale;
}

export function width(n) {
  return n + "px";
}

export function willCoercionThrow(value) {
  try {
    "" + value;
    return false;
  } catch {
    return true;
  }
}

export function requireString() {
  return "require" + Math.random();
}

export function nodeRequire(module, key) {
  return module && module[key];
}
```

`js::shown(value)` displays any value as JS's template shows it,
`` `${value}` ``: an object by its `toString`, a number as JS writes it,
`1e+21`. Its text is a string wherever it goes, alone too, padded, and by
`to_string()` (ADR 0312):

```rust
use js::Unknown;

pub fn key(msg: Option<&Unknown>, index: usize) -> String {
    format!("{}-{index}", js::shown(msg))
}

pub fn year(year: f64) -> String {
    format!("{}", js::shown(year))
}

pub fn year_width(year: f64) -> usize {
    js::shown(year).to_string().len()
}
```

```js
import { $byteLen } from "@rust-js/runtime";

export function key(msg, index) {
  return `${msg}-${index}`;
}

export function year(year) {
  return `${year}`;
}

export function yearWidth(year) {
  return $byteLen(`${year}`);
}
```

## Parameters and results

```rust
use std::marker::PhantomData;

use js::{IntoUnknown, JsError, JsObject, Promise, Rest, Unknown};

pub struct Editor(PhantomData<JsObject>);

#[derive(Default)]
pub struct EditorOptions<'a> {
    pub theme: &'a str,
    pub line_numbers: Option<bool>,
    pub tab_size: Option<u32>,
}

#[cfg_attr(rust_js, rust_js::link_name = "new my-editor#Editor")]
pub fn create(options: EditorOptions<'_>) -> &'static Editor {
    unreachable!()
}

/// Throws a `SyntaxError` of text it can't parse.
#[cfg_attr(rust_js, rust_js::link_name = "my-editor#parse")]
pub fn parse(source: &str) -> Result<&'static Unknown, &'static JsError> {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "my-editor#load")]
pub fn load(url: &str) -> Promise<String> {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "my-editor#onChange")]
pub fn on_change(editor: &Editor, listener: Box<dyn Fn(&str)>) {
    unreachable!()
}

/// `number | null`: where `text` is, or `null`.
#[cfg_attr(rust_js, rust_js::link_name = "my-editor#find")]
#[cfg_attr(rust_js, rust_js::nullable)]
pub fn find(editor: &Editor, text: &str) -> Option<u32> {
    unreachable!()
}

/// `range: [number, number] | null`: `null` clears the selection.
#[cfg_attr(rust_js, rust_js::link_name = "my-editor#select")]
#[cfg_attr(rust_js, rust_js::nullable(range))]
pub fn select(editor: &Editor, range: Option<(u32, u32)>) {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "instanceof my-editor#Editor")]
pub fn is_editor(this: &Unknown) -> bool {
    unreachable!()
}

/// `value` as an `Editor`, unchecked.
///
/// # Safety
///
/// `value` must be an `Editor`.
#[cfg_attr(rust_js, rust_js::link_name = "this")]
pub unsafe fn unchecked_editor(this: &Unknown) -> &Editor {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "console.log")]
#[cfg_attr(rust_js, rust_js::variadic)]
pub fn log(data: impl Rest<dyn IntoUnknown>) {
    unreachable!()
}

pub async fn open(other: &Unknown) -> Result<(), &'static JsError> {
    let editor = create(EditorOptions {
        theme: "dark",
        tab_size: Some(4),
        ..Default::default()
    });
    on_change(editor, Box::new(|text| println!("{text}")));
    let source = load("/main.rs").await;
    let tree = parse(&source)?;
    match find(editor, "fn") {
        Some(at) => select(editor, Some((at, at + 2))),
        None => select(editor, None),
    }
    if is_editor(other) {
        let other = unsafe { unchecked_editor(other) };
        select(other, None);
    }
    log(("parsed", js::type_of(Some(tree))));
    Ok(())
}
```

```js
import { Editor, find, load, onChange, parse, select } from "my-editor";
import { $try } from "@rust-js/runtime";

export async function open(other) {
  const editor = new Editor({
    theme: "dark",
    tabSize: 4,
  });
  onChange(editor, (text) => {
    console.log(`${text}`);
  });
  const source = await load("/main.rs");
  const result = $try(() => parse(source));
  if (result.TAG === "Err") {
    return result;
  }
  const tree = result._0;
  const at = find(editor, "fn");
  if (at != null) {
    select(editor, [at, (at + 2) >>> 0]);
  } else {
    select(editor, null);
  }
  if (other instanceof Editor) {
    select(other, null);
  }
  console.log("parsed", typeof tree);
  return { TAG: "Ok" };
}
```

- **Values are as [JavaScript types](types.md) has them**: a `&str` is a
  string, a tuple an array, a struct an object.
- **An options object is a struct of `Option` fields**, the rest
  `..Default::default()`. A `None` field is left out. One of any value,
  TypeScript's `any`, is an `Option<&dyn IntoUnknown>`, given as it is:
  `ErrorEventInit { error: Some(&error), .. }` is `{ error }`
  ([ADR 0654](../../decisions/0654-a-dictionary-member-of-any-value-is-a-dyn-into-unknown.md)).
- **What may throw returns `Result<T, &JsError>`**: the call runs in a `try`,
  and what JS threw is the `Err`. A binding that doesn't say so lets a throw
  through, as a panic ([Errors, panics and exceptions](exceptions.md)).
- **A promise is a `Promise<T>`**, to `.await` ([Promises and async](async.md)).
- **A callback is a closure**: `Box<dyn Fn(..)>` for one JS may call again,
  `Box<dyn FnOnce()>` for one it calls once, as the js crate's
  `set_timeout` and webapi's `queue_microtask` and
  `request_animation_frame` take, so it may move out what it holds. JS
  calls it with its parameters ([ADR 0260](../../decisions/0260-webapi-callbacks.md),
  [ADR 0869](../../decisions/0869-a-web-callback-called-once-is-fn-once.md)).
  JS keeps it, and has no drop: a closure boxed as it's given to a
  binding, `on_change(editor, Box::new(move || ..))`, drops nothing of
  what it holds after, and carries no drop. One Rust still holds drops as
  Rust drops it: as a panic unwinds before it's handed over, or where it
  never is; and one a binding gives back, as `widen` does, is Rust's
  again ([ADR 0873](../../decisions/0873-a-function-handed-to-js-carries-no-drop.md)).
- **A function JS gives is a `&'static dyn Fn` of its signature**, its
  `any` an `Option<&Unknown>`: `button.onclick()` is an
  `Option<&'static dyn Fn(&PointerEvent) -> Option<&'static Unknown>>`, to
  call, keep, or compare by `std::ptr::eq`, which is `===`
  ([ADR 0653](../../decisions/0653-a-js-function-value-is-a-dyn-fn-of-its-signature.md)).
  TypeScript's `Function`, of whatever it takes, is `&dyn Fn() ->
  Option<&'static Unknown>`, called of nothing for any value, as
  `js::Kind::Function` holds it
  ([ADR 0700](../../decisions/0700-typescripts-function-gives-any-value.md)).
- **`#[rust_js::nullable]` makes the result's `None` `null`** for a caller
  that tests `!== null`; `#[rust_js::nullable(range)]` passes the parameter
  `range`'s `None` as `null`. Of a type parameter, `fn create_cursor<T>(
  default_value: T)`, it's where it's given an `Option`, `createCursor(null)`,
  and any other value as it is, `createCursor(0)`
  ([ADR 0586](../../decisions/0586-a-nullable-field-is-an-option.md), amended).
  Reading a result, `None` is `undefined` or `null` either way
  ([ADR 0275](../../decisions/0275-nullable-writes-null.md)).
- **`#[rust_js::variadic]` makes the last parameter the rest
  arguments** ([ADR 0221](../../decisions/0221-variadic-bindings.md)). A
  TypeScript rest parameter, `log(...data: any[])`, is `impl js::Rest<T>`
  ([ADR 0640](../../decisions/0640-a-rest-parameter-is-a-tuple.md)):
  given a tuple, its arguments, each of its own type that a `T` takes,
  `dyn IntoUnknown` of `any`, `str` of `string`, `f64` of `number`, which
  takes any number JS has as one, each `f64::from` takes, `(x,)` one and
  `()` none; given a slice, a `Vec` or a typed array by reference, spread,
  `log(&values)` being `log(...values)` and `string::from_char_code(bytes)`
  `String.fromCharCode(...bytes)`
  ([ADR 0718](../../decisions/0718-a-typed-array-is-rest-arguments-spread.md)),
  and so is rest arguments as a value, a `&dyn js::Rest<T>`
  ([above](#a-binding-as-a-value)). A slice declared is refused.
- **A cast is `this`, unchecked, so it's an `unsafe fn`**; check first with
  `instanceof`. webapi has one for each interface,
  `html_input_element::unchecked_from(target)`. A `let` of a variable
  cast, `let lazy: &LazyComponent = unsafe { js::cast(children) };`, is
  that variable where nothing reads it after, `children._payload`, as
  TypeScript's `(children as any)._payload` reads; else its own name,
  `const lazy = children;`
  ([ADR 0598](../../decisions/0598-a-temporary-is-no-part-of-a-names-pattern.md), amended).
  `js::cast` vouches the value is what JS has of its type: one that may be
  `undefined`, `js::get(timers, "setImmediate")`, is cast to an `Option`,
  `unsafe { js::cast::<Option<fn(..)>>(..) }`. `Some(js::cast(v))` of an
  `undefined` is that `Option`'s `None`, as JS holds `Some` of a function
  as the function itself, `undefined` its `None`: no check says it isn't
  what it was vouched to be.

### Optional parameters

A parameter a caller may leave out, TypeScript's `x?`, is an `Option`. A
trailing `None` is no argument, as a person leaves it out:
`chart.update(None)` is `chart.update()`
([ADR 0443](../../decisions/0443-a-value-a-caller-may-leave-out-is-an-option.md)).
A setter's value is never left out: a `set x` binding's `None` is `x =
undefined`, `Error.prepareStackTrace = undefined`
([ADR 0648](../../decisions/0648-v8s-stack-trace-api-is-the-node-crates.md)).

There's one Rust function for each JS function. Don't write a second one
for fewer arguments, `update_with_mode`: give the parameter an `Option`.
Separate functions are for signatures that differ in a parameter's type or
in their result, not only in how many arguments they take.

An optional parameter of a generic type is taken by reference, so a bare
`None` needs no type:

```rust
use js::{IntoUnknown, Promise};

/// `put(key: string, value?: any): Promise<void>`.
#[cfg_attr(rust_js, rust_js::link_name = "my-store#put")]
pub fn put(key: &str, value: Option<&dyn IntoUnknown>) -> Promise<()> {
    unreachable!()
}

pub async fn fill() {
    put("count", Some(&3)).await;
    put("empty", None).await;
}
```

```js
import { put } from "my-store";

export async function fill() {
  await put("count", 3);
  await put("empty");
}
```

### Unions and `any`

A parameter that's a union, TypeScript's `string | number`, takes `impl` a
trait each member implements. The value is passed as it is
([ADR 0229](../../decisions/0229-union-parameters.md)):

```rust
# use std::marker::PhantomData;
#
# use js::JsObject;
#
# pub struct Shape(PhantomData<JsObject>);
#
/// What `string | number` takes.
#[cfg_attr(rust_js, rust_js::types = "string | number")]
pub trait IntoLabel {}
impl IntoLabel for &str {}
impl IntoLabel for f64 {}

impl Shape {
    /// `label(text: string | number)`.
    #[cfg_attr(rust_js, rust_js::link_name = "label")]
    pub fn label(&self, text: impl IntoLabel) {
        unreachable!()
    }
}
```

`c.label("unit")` and `c.label(1.5)` are `c.label("unit")` and `c.label(1.5)`;
`c.label(true)` is rustc's error. A union that's returned, or a field, is an
[untagged enum](types.md#enums), to `match`.

A parameter of TypeScript's `any` or `unknown` is generic:

| Takes | Rust |
|---|---|
| any value that's never `undefined` or `null` | `value: &T`, `T: js::Defined + ?Sized` |
| any value, left out or not | `value: Option<&dyn js::IntoUnknown>` |
| a value the structured clone copies, `postMessage`'s, `()` its `undefined` | `value: T`, `T: js::StructuredClone` |
| a value whose type you don't know | `&js::Unknown`, or `Option<&js::Unknown>` |

A `()` given any of them is `undefined`, what `()` is everywhere:
`js::strict_equals(x, ())` is `x === undefined`. Only a list's `()` is
the list of none: rest arguments', none given, and a hook's
dependencies', `use_memo(f, ())`'s `useMemo(f, [])`
([ADR 0775](../../decisions/0775-a-unit-given-to-js-is-undefined.md)).

```rust
use js::{Defined, StructuredClone};

/// `inspect(value: unknown)`.
#[cfg_attr(rust_js, rust_js::link_name = "shapes#inspect")]
pub fn inspect<T: Defined + ?Sized>(value: &T) {
    unreachable!()
}

/// `save(value)`, which posts a structured clone of it to a worker.
#[cfg_attr(rust_js, rust_js::link_name = "shapes#save")]
pub fn save<T: StructuredClone>(value: T) {
    unreachable!()
}

pub struct Saved {
    pub radius: f64,
}

unsafe impl StructuredClone for Saved {}
```

`save(Saved { radius: 2.0 })` is `save({ radius: 2 })`. A struct is a
`StructuredClone` by an `unsafe impl` that vouches each field is: a closure
or a DOM node isn't.

## Bind a component

A React component of an npm package is a binding of its props, as a
component of your own is a function of them:

```rust
use react::{JSX, jsx};

pub struct ToasterProps {
    pub position: &'static str,
    pub rich_colors: Option<bool>,
}

#[cfg_attr(rust_js, rust_js::link_name = "sonner#Toaster")]
pub fn Toaster(props: ToasterProps) -> JSX::Element {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "sonner#toast")]
pub fn toast(message: &str) {
    unreachable!()
}

pub fn App() -> JSX::Element {
    jsx! {
        <main>
            <button onClick={|_| toast("Saved")}>"Save"</button>
            <Toaster position="top-right" />
        </main>
    }
}
```

```jsx
import { Toaster, toast } from "sonner";

export function App() {
  return (
    <main>
      <button onClick={() => toast("Saved")}>Save</button>
      <Toaster position="top-right" />
    </main>
  );
}
```

See [Components and props](../react/components.md).

## Other attributes of a binding

| Attribute | Says |
|---|---|
| `#[rust_js::nullable]`, `#[rust_js::nullable(param)]` | its result's, or that parameter's, `None` is `null` |
| `#[rust_js::variadic]` | its last parameter, `impl js::Rest<T>` or a tuple, is the rest arguments |
| `#[rust_js::require]` | it's read through `require(module)`, not imported |
| `#[rust_js::cannot_throw]` | it can't throw, as reading a data property can't |
| `#[rust_js::runs_nothing]` | it runs no code but its own, not even what it's given, as `new AggregateError(errors)` doesn't: a `RefCell` borrowed across it isn't counted ([ADR 0867](../../decisions/0867-a-binding-that-runs-nothing-and-aggregate-error.md)) |
| `#[rust_js::runs_only_given]` | it runs no code but its own and what it's given, a closure it calls or a `toString` or getter of a value it reads, as `console.error(..)` and `Math.max(..)` do: given only numbers, `bool`s and text, a `RefCell` borrowed across it isn't counted. A program that replaces such a function with its own is trusted not to borrow what's held across it ([ADR 0898](../../decisions/0898-a-binding-that-runs-only-what-its-given.md)) |
| `#[rust_js::skips_falsy]` | it skips what's falsy, as `classnames` does: `test && value` |
| `#[rust_js::position]` | it gives a position, or `-1`, as `indexOf` does |
| `#[rust_js::returns_undefined]` | it gives `undefined`, whatever its TypeScript type says; the webapi crate's say it of each operation WebIDL says gives `undefined` ([ADR 0883](../../decisions/0883-a-closure-of-its-calls-value-is-an-arrow-of-it.md)) |
| `#[rust_js::test = "react#isValidElement"]` | of a type: the function that tells one, where no class does |

A function of yours that JS calls with rest arguments is the other side
of `#[rust_js::variadic]`: its last parameter, a slice, is marked
`#[cfg_attr(rust_js, rust_js::rest)]`, `function f(code, ...args)`
([Rest parameters](export-to-javascript.md#rest-parameters)).

## How much the crates bind

Each crate lists what it binds against TypeScript's own declarations, member
by member: `webapi/coverage.txt` against the DOM's, `builtins/coverage.txt`
(the `js` crate) against ES2024's, `node/coverage.txt` against
`@types/node`, `react/coverage.txt` against `@types/react`. A member of
TypeScript's that a crate lacks, or types more loosely, is a gap in rust-js:
report it, and don't bind it yourself in your app.
