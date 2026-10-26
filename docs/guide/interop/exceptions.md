# Errors, panics and exceptions

Rust has two kinds of failure, and each is what JS would make of it: a
`Result` is a value you return, and a panic is a thrown `Error`. A JS
exception reaches Rust as an `Err`, where you ask for one, or unwinds it
as a panic does, which `catch_unwind` catches.

Every example was compiled by rust-js as it is today. The JS shown is what
it writes, without its header comment and source-map line.

## Contents

- [A panic is a thrown `Error`](#a-panic-is-a-thrown-error)
- [`Result` and `?`](#result-and-)
- [Catch a JS exception](#catch-a-js-exception)
- [Unwinding runs `Drop`](#unwinding-runs-drop)
- [`RefCell` borrows](#refcell-borrows)
- [Arithmetic and indexing](#arithmetic-and-indexing)
- [Assertions, `unreachable!` and `todo!`](#assertions-unreachable-and-todo)
- [Catch an unwind: `catch_unwind`](#catch-an-unwind-catch_unwind)
- [Throw a JS value: `js::throw`](#throw-a-js-value-jsthrow)
- [Exiting the process](#exiting-the-process)

## A panic is a thrown `Error`

```rust
pub fn third(items: &[u32]) -> u32 {
    items[2]
}

pub fn port(text: &str) -> u16 {
    text.parse().expect("a port number")
}

pub fn ratio(a: u32, b: u32) -> u32 {
    a / b
}

pub fn fail(message: &str) -> ! {
    panic!("{message}")
}
```

```js
import { $debugParseError, $div, $index, $parseInt, $unwrapOk } from "@rust-js/runtime";

export function third(items) {
  return $index(items, 2);
}

export function port(text) {
  return $unwrapOk($parseInt(text, 0, 65535), "a port number", (e) =>
    $debugParseError(e, "ParseIntError"),
  );
}

export function ratio(a, b) {
  return $div(a, b) >>> 0;
}

export function fail(message) {
  throw new Error(message);
}
```

Each throws a plain `Error` whose `message` is Rust's panic message:

| Rust | `error.message` |
|---|---|
| `panic!("{message}")` | the message |
| `items[2]` of one item | `index out of bounds: the len is 1 but the index is 2` |
| `.expect("a port number")` of an `Err` | `a port number: ParseIntError { kind: InvalidDigit }`, the error's `{:?}` |
| `.unwrap()` of an `Err` | ``called `Result::unwrap()` on an `Err` value: NotANumber("x")`` |
| `.unwrap()` of `None` | ``called `Option::unwrap()` on a `None` value`` |
| `a / b` of `b == 0` | `attempt to divide by zero` |

## `Result` and `?`

A `Result` is an object, `{ TAG: "Ok", _0: v }` or `{ TAG: "Err", _0: e }`,
and `?` returns its `Err` early, as Rust's does:

```rust
#[derive(Debug)]
pub enum ParseError {
    Empty,
    NotANumber(String),
}

pub fn parse(text: &str) -> Result<u32, ParseError> {
    if text.is_empty() {
        return Err(ParseError::Empty);
    }
    text.parse()
        .map_err(|_| ParseError::NotANumber(text.to_string()))
}

pub fn sum(a: &str, b: &str) -> Result<u32, ParseError> {
    Ok(parse(a)? + parse(b)?)
}

pub fn must(text: &str) -> u32 {
    parse(text).unwrap()
}
```

```js
import { $debugStr, $parseInt, $unwrapOk } from "@rust-js/runtime";

function parseErrorDebug_fmt(parseError) {
  if (parseError === "Empty") {
    return "Empty";
  }
  return `NotANumber(${$debugStr(parseError._0)})`;
}

export function parse(text) {
  if (!text) {
    return { TAG: "Err", _0: "Empty" };
  }
  const result = $parseInt(text, 0, 4294967295);
  return result.TAG === "Err" ? { TAG: "Err", _0: { TAG: "NotANumber", _0: text } } : result;
}

export function sum(a, b) {
  const result = parse(a);
  if (result.TAG === "Err") {
    return result;
  }
  const _0 = result._0;
  const result$1 = parse(b);
  if (result$1.TAG === "Err") {
    return result$1;
  }
  return { TAG: "Ok", _0: (_0 + result$1._0) >>> 0 };
}

export function must(text) {
  return $unwrapOk(parse(text), undefined, parseErrorDebug_fmt);
}
```

- **A JS caller reads the `TAG`**: `sum("1", "x")` is
  `{ TAG: "Err", _0: { TAG: "NotANumber", _0: "x" } }`, not a throw.
- **`?` converts the error with your `From` impls**, as Rust's does.
  `Box<dyn Error>` takes your errors, strings (`"too big".into()`) and
  std's parse errors.
- **`?` on an `Option`** returns `None`, `undefined`.

## Catch a JS exception

A binding whose result is `Result<T, &JsError>` catches what its JS
function throws: its `Err` is the thrown value ([ADR 0035](../../decisions/0035-results-and-throwing-js.md)). Any other
binding lets it propagate, through your Rust, as a panic would.

```rust
use js::JsError;

#[cfg_attr(rust_js, rust_js::link_name = "./config.js#loadConfig")]
pub fn load_config(path: &str) -> Result<String, &'static JsError> {
    unreachable!()
}

pub fn config_or_default(path: &str) -> String {
    load_config(path).unwrap_or_default()
}

pub fn config(path: &str) -> String {
    load_config(path).expect("the config loads")
}

pub fn loaded(path: &str) -> String {
    load_config(path).unwrap()
}

fn read_config(path: &str) -> Result<String, &'static JsError> {
    load_config(path)
}

pub fn readable(path: &str) -> bool {
    read_config(path).is_ok()
}

pub fn decoded(text: &str) -> String {
    match js::decode_uri_component(text) {
        Ok(decoded) => decoded,
        Err(error) => format!("{}: {}", error.name(), error.message()),
    }
}

pub fn config_length(path: &str) -> Option<usize> {
    match load_config(path) {
        Ok(text) => Some(text.trim().len()),
        Err(_) => None,
    }
}

pub fn has_config(path: &str) -> bool {
    load_config(path).is_ok()
}
```

```js
import { loadConfig } from "./config.js";
import { $byteLen, $trim, $try, $unwrapOk } from "@rust-js/runtime";

export function configOrDefault(path) {
  try {
    return loadConfig(path);
  } catch {
    return "";
  }
}

export function config(path) {
  return $unwrapOk(
    $try(() => loadConfig(path)),
    "the config loads",
    String,
  );
}

export function loaded(path) {
  return loadConfig(path);
}

function readConfig(path) {
  return loadConfig(path);
}

export function readable(path) {
  try {
    readConfig(path);
    return true;
  } catch {
    return false;
  }
}

export function decoded(text) {
  try {
    return decodeURIComponent(text);
  } catch (error) {
    return `${error.name}: ${error.message}`;
  }
}

export function configLength(path) {
  let text;
  try {
    text = loadConfig(path);
  } catch {
    return;
  }
  return $byteLen($trim(text));
}

export function hasConfig(path) {
  try {
    loadConfig(path);
    return true;
  } catch {
    return false;
  }
}
```

- **A `match` of the `Result` is `try` and `catch`** of the call, the
  `Err`'s binding the `catch`'s, where its `Err` arm reads the error by
  that binding alone ([ADR 0637](../../decisions/0637-a-match-of-a-throwing-call-is-try-catch.md)).
  Only the call is in the `try`, as only it is caught in Rust: what the
  `Ok` arm does is in it where it can't throw, and after it where it may
  and the `Err` arm leaves, its value a `let` the `try` sets. Whether it
  threw, returned, `is_ok()` or `is_err()` or a `match` of `true` and
  `false`, is `try { f(); return true; } catch { return false; }`
  ([ADR 0639](../../decisions/0639-concat-text-of-any-value-may-throw.md)).
  Any other use of the `Result` is `$try`'s `{ TAG, _0 }`.
- **`unwrap()` of the call is the call**, `loadConfig(path)`: what JS
  threw goes on as itself, not as Rust's panic of it. `expect(msg)` keeps
  its message ([ADR 0643](../../decisions/0643-unwrap-of-a-throwing-call-is-the-call.md)).
  So is each branch's call of an `if` or a `match` unwrapped where it's
  made, `if c { f(a) } else { f(b) }.unwrap()`, `c ? f(a) : f(b)`; a
  `Result` held while something else runs keeps `$try`'s
  ([ADR 0747](../../decisions/0747-unwrap-of-a-throwing-call-each-branch-makes.md)).
- **A function whose result is the call's `Result` throws too**,
  `readConfig(path)`, which returns `loadConfig(path)`: its calls are
  throwing calls, by each rule above. Not a `pub` one, whose JS callers
  read its `Result`, nor a trait's method or a generic one
  ([ADR 0644](../../decisions/0644-a-function-of-a-throwing-calls-result-throws.md)).
- **`&JsError` is whatever was thrown**: `message()`, `name()`,
  `cause()` and `stack()` read an `Error`'s; `is_error()` says whether it
  is one, since JS may throw anything. Its `{:?}` is `String(error)`,
  `TypeError: no file missing`, what an `expect` shows. `as_unknown()` is
  it as a value of any shape, an `Option<&Unknown>`, the value itself,
  `None` of `undefined` and `null`, which a parameter or a field of any
  value takes, `node.v = error` of `node.v.set(error.as_unknown())`
  ([ADR 0782](../../decisions/0782-a-promises-executor-borrows-and-then-takes-two-handlers.md#amendment-what-was-caught-is-a-value-of-any-shape-as_unknown)).
  How V8 writes a `stack` is a hook of the node crate's,
  `Error.prepareStackTrace` ([`node`](../libraries/README.md#node-nodejs)).
- **Several errors as one is an `AggregateError`**:
  `js::aggregate_error::new(errors, message)` is `new
  AggregateError(errors, message)`, `errors` a slice of any value,
  `&[Option<&Unknown>]`, and `message` an `Option<&str>`; its `errors()`
  is what it holds, and it's a `&JsError` by `Deref`, `as_unknown()` the
  value to throw. Making one runs none of your code, so a `RefCell`
  borrowed for it isn't counted ([ADR 0867](../../decisions/0867-a-binding-that-runs-nothing-and-aggregate-error.md)).
- **The crates say which JS calls may throw.** The `js` crate's
  `decode_uri_component` and `Json::parse` return a `Result`, and
  `concat_text`, `"" + value`, of a value of any shape. webapi's
  call that throws for what a program can't prevent returns one, as
  `window.local_storage()` does where the page may not keep data; one
  that throws only of a misuse, an invalid selector, panics ([ADR 0442](../../decisions/0442-a-web-api-that-may-throw-is-a-result.md)).
- **A rejected promise** is the same: see
  [Promises and async](async.md#rejections).

How to write a binding is [Write bindings for JavaScript APIs](bindings.md).

## Unwinding runs `Drop`

A panic unwinds as Rust's does: each value with a destructor in a scope
it leaves is dropped, innermost first, through a `try`/`finally` or a
`catch` that throws again ([ADR 0437](../../decisions/0437-a-scopes-drops-are-one-finally.md)):

```rust
pub struct Guard {
    pub name: &'static str,
}

impl Drop for Guard {
    fn drop(&mut self) {
        println!("closing {}", self.name);
    }
}

pub fn work(fail: bool) -> u32 {
    let _outer = Guard { name: "outer" };
    let _inner = Guard { name: "inner" };
    if fail {
        panic!("failed");
    }
    1
}
```

```js
export function work(fail) {
  const _outer = { name: "outer" };
  const name = "inner";
  try {
    try {
      if (fail) {
        throw new Error("failed");
      }
    } finally {
      console.log(`closing ${name}`);
    }
  } finally {
    console.log(`closing ${_outer.name}`);
  }
  return 1;
}
```

`work(true)` prints `closing inner`, then `closing outer`, then throws.
A JS exception thrown through Rust code unwinds it the same way. A `drop`
that panics while another panic unwinds replaces that panic, where Rust
aborts.

A scope's `if`s, loops and `return`s from inside them are in its `try`,
`if (wrap) { return callback(); }` too, whose `finally` drops what it
holds however it's left, after what's returned is made: a guard whose
`Drop` restores state is a `finally` ([ADR 0693](../../decisions/0693-a-guards-scope-is-one-finally-its-returns-in-it.md)),
one `try`, `catch` and `finally` around a `catch_unwind`'s `catch`, a
function or a constant it returns last after them, `return noop;`, as the
drop can't change it, and the drop's body is in it, what it declares
named apart from the function's where JS's scopes need it ([ADR 0608](../../decisions/0608-a-drop-is-its-body-at-each-drop.md)).
The scope reads as it would without the guard, and the guard's fields
are the variables it was made of, `name` above, a tuple struct's one
field too, `Restore(previous)` read `previous`, where nothing sets them
once it's made ([ADR 0705](../../decisions/0705-a-guards-scope-reads-as-any-body.md)).

## `RefCell` borrows

A `RefCell` counts its borrows, as std's does, and a borrow it can't
give panics ([ADR 0328](../../decisions/0328-refcell-borrow-counts.md)):

```rust
use std::cell::RefCell;

pub fn record(log: &RefCell<Vec<String>>) {
    let entries = log.borrow();
    log.borrow_mut().push(format!("{} entries", entries.len()));
}
```

```js
import { $borrow, $borrowMut, $unborrow } from "@rust-js/runtime";

export function record(log) {
  const entries = $borrow(log, true);
  try {
    const borrowMut = $borrowMut(log, true);
    try {
      borrowMut.push(`${entries.length} entries`);
    } finally {
      $unborrow(borrowMut);
    }
  } finally {
    $unborrow(entries);
  }
}
```

`record` throws `RefCell already borrowed`, as Rust panics. A
`borrow()` while it's mutably borrowed is `RefCell already mutably
borrowed`; `try_borrow()` and `try_borrow_mut()` give that as an `Err`.
A borrow made and dropped in one statement, where nothing could ask, is
checked and not counted. A `Mutex` and an `RwLock` count their guards the
same way.

## Arithmetic and indexing

```rust
pub fn add(a: u8, b: u8) -> u8 {
    a + b
}

pub fn checked_add(a: u8, b: u8) -> Option<u8> {
    a.checked_add(b)
}
```

```js
import { $checked } from "@rust-js/runtime";

export function add(a, b) {
  return (a + b) & 255;
}

export function checkedAdd(a, b) {
  return $checked(a + b, 0, 255);
}
```

- **Integer overflow wraps**, as a release build's does, in every build:
  `add(200, 100)` is `44`. Ask for the check you need, `checked_add`,
  `None` there, or `wrapping_add` and `saturating_add`.
- **Division by zero panics**, `attempt to divide by zero`, and so does
  `i32::MIN / -1`, `attempt to divide with overflow`.
- **An index out of bounds panics**, of an array, a `Vec` or a slice.
  `get(i)` gives `None` instead.

## Assertions, `unreachable!` and `todo!`

```rust
pub fn pick(n: u32) -> &'static str {
    match n % 3 {
        0 => "fizz",
        1 => "buzz",
        2 => "none",
        _ => unreachable!("a remainder of 3 is under 3"),
    }
}

pub fn check(items: &[u32], expected: u32) {
    assert!(items.len() < 10, "too many items: {}", items.len());
    assert_eq!(items.iter().sum::<u32>(), expected);
}
```

```js
import { $assertFailed } from "@rust-js/runtime";

export function pick(n) {
  switch (n % 3) {
    case 0:
      return "fizz";
    case 1:
      return "buzz";
    case 2:
      return "none";
    default:
      throw new Error("internal error: entered unreachable code: a remainder of 3 is under 3");
  }
}

export function check(items, expected) {
  if (items.length >= 10) {
    throw new Error(`too many items: ${items.length}`);
  }
  const value = items.reduce((a, b) => (a + b) >>> 0, 0);
  const value$1 = expected;
  if (value !== value$1) {
    $assertFailed("Eq", String(value), String(value$1));
  }
}
```

- `assert_eq!` fails with Rust's message, both sides shown:

  ```text
  assertion `left == right` failed
    left: 3
   right: 4
  ```

- `todo!()` throws `not yet implemented`, `unimplemented!()` `not
  implemented`.
- `debug_assert!` is checked, as `assert!` is.

## Catch an unwind: `catch_unwind`

A panic is an ordinary `Error`, so JS that calls Rust catches it:

```js
import { work } from "./lib.js";

try {
  work(true);
} catch (error) {
  console.log(error.message); // "failed"
}
```

An uncaught one is the host's error: a stack trace in the console, and in
Node exit code 1, where native Rust prints `thread 'main' panicked at` and
exits with 101.

Rust catches an unwind with `std::panic::catch_unwind`, and an unwind is
whatever JS throws through your Rust: a panic, or what a getter, a
binding or a callback threw ([ADR 0660](../../decisions/0660-catch-unwind-catches-what-js-throws.md)).
Where its `Result` is matched, its `unwrap_or` returned, or an `if let
Ok(v)` returns `v`, it's JS's `try`, the closure's body in it; the last's
`catch` does nothing, what follows running where it threw. An `Ok` arm
is in the `try` where it can't throw, one that tests what it holds too,
`Ok(Some(f)) => return f`, the value by its pattern's name, `if (f) {
return f; }`; one that may, a binding's setter's among it,
runs after the `try`, as what it throws isn't the `catch`'s:

```rust
use js::{JsError, Kind, Unknown};
use std::panic;

pub fn url(value: &Unknown) -> String {
    panic::catch_unwind(|| match js::classify(js::get(value, "url")) {
        Kind::String(url) => url.to_string(),
        _ => String::new(),
    })
    .unwrap_or_default()
}

pub fn item(items: &[u32], at: usize) -> String {
    match panic::catch_unwind(|| items[at]) {
        Ok(item) => item.to_string(),
        Err(payload) => match payload.downcast_ref::<String>() {
            Some(message) => message.clone(),
            None => "a panic".to_string(),
        },
    }
}

pub fn first_text(values: &[&Unknown]) -> String {
    for &value in values {
        if let Ok(text) = panic::catch_unwind(|| js::concat_text(value).unwrap()) {
            return text;
        }
    }
    String::new()
}

pub fn text(value: &Unknown) -> String {
    match panic::catch_unwind(|| js::concat_text(value).unwrap()) {
        Ok(text) => text,
        Err(payload) => {
            if payload.downcast_ref::<&'static JsError>().is_some_and(|error| error.name() == "TypeError") {
                return String::new();
            }
            panic::resume_unwind(payload)
        }
    }
}
```

```js
import { $index, $panicMessage, $unwound } from "@rust-js/runtime";

export function url(value) {
  try {
    const url = value.url;
    if (typeof url === "string") {
      return url;
    }
    return "";
  } catch {
    return "";
  }
}

export function item(items, at) {
  try {
    const item = $index(items, at);
    return String(item);
  } catch (thrown) {
    const payload = $unwound(thrown);
    const message =
      payload.impl.typeId() === "alloc::string::String" || payload.impl === $panicMessage
        ? payload.value
        : undefined;
    if (message != null) {
      return message;
    }
    return "a panic";
  }
}

export function firstText(values) {
  for (const value of values) {
    try {
      return "" + value;
    } catch {}
  }
  return "";
}

export function text(value) {
  try {
    return "" + value;
  } catch (payload) {
    if (payload.name === "TypeError") {
      return "";
    }
    throw payload;
  }
}
```

- **The payload is a `Box<dyn Any + Send>`**: a panic's message, which
  downcasts as a `&str` and as a `String` both, as JS doesn't keep which
  Rust's was; or what JS threw, a `&'static JsError`. A thrown `Error` of
  JS's own constructor, `new Error(..)`, is a panic, whoever threw it, and
  what JS threw too: its message downcasts as a `&str` and a `String`, and
  the `Error` itself, its `stack`, as a `&JsError` ([ADR 0692](../../decisions/0692-a-thrown-error-is-a-panic-and-what-js-threw.md)).
- **Every payload downcasts as a `&'static JsError` too, what was
  thrown**: a panic's `Error`, what JS threw, a falsy value among it, or
  the `Error` a payload of another type, `resume_unwind(Box::new(5))`'s,
  is thrown in, which still downcasts to its own type. So the `Some` of
  `downcast_ref::<&JsError>()` is what's matched, its `None` arm never
  taken and left out, and the `catch` is of what was thrown ([ADR 0710](../../decisions/0710-every-unwinds-payload-is-the-js-error-thrown.md)).
  What's given its value, the closure of `and_then`, `map` or
  `is_some_and`, is given it, as a `Some`'s: `x.stack`, `payload.name`
  above, a falsy one's too. A falsy one's `Option` carried on as a value,
  `.copied()`, `.is_some()`, is `None`, as JS's `if (x)` reads it.
- **`resume_unwind(payload)` throws it again**: a thrown `Error` or what
  JS threw, itself; text, its panic; any other payload, an `Error` that a
  `catch_unwind` gives back as it was. Of a payload only read, it's
  `throw` of what was caught, the same object, `throw thrown` above.
- **`panic!` is the one way to throw a new `Error`**, `panic!("")` a `new
  Error()`: a JS value isn't `Send`, so rustc refuses `resume_unwind`
  and `panic_any` of a `&JsError`; a value JS already has is thrown by
  [`js::throw`](#throw-a-js-value-jsthrow). Caught, its `&JsError` is that
  `Error`, its `stack` where it was thrown, and the `catch` is of the
  `Err`'s binding ([ADR 0706](../../decisions/0706-panic-is-the-one-way-a-port-throws-an-error.md)).
  An `if let Err(e) = catch_unwind(f)` that heads a chain, `&& c`, is a
  `try` too, the rest of the chain the `catch`'s `if`, which reads what
  the closure set; its `else`, where the `if` returns, what follows the
  `try`:

```rust
use js::{JsError, Unknown};
use std::panic::{AssertUnwindSafe, catch_unwind};

pub fn stacks(render: &dyn Fn()) -> Option<(String, Option<String>)> {
    let mut control = None;
    if let Err(sample) = catch_unwind(AssertUnwindSafe(|| {
        let Err(x) = catch_unwind(|| panic!(""));
        control = x.downcast_ref::<&'static JsError>().copied();
        render();
    }))
        && let Some(sample) = sample.downcast_ref::<&'static JsError>()
        && let Some(control) = control
        && let Some(sample_stack) = sample.stack()
    {
        return Some((sample_stack, control.stack()));
    }
    None
}

pub fn control_stack() -> Option<String> {
    let Err(x) = catch_unwind(|| panic!(""));
    let control = x.downcast_ref::<&'static JsError>().copied();
    control.and_then(|error| error.stack())
}

pub fn described(value: &Unknown) -> String {
    match catch_unwind(|| js::concat_text(value).unwrap()) {
        Ok(text) => text,
        Err(x) => match x.downcast_ref::<&'static JsError>() {
            Some(x) => format!("\nError describing: {}\n{}", x.message(), x.stack().unwrap_or_default()),
            None => String::from("\nError describing"),
        },
    }
}
```

```js
export function stacks(render) {
  let control;
  try {
    try {
      throw new Error();
    } catch (x) {
      control = x;
    }
    render();
  } catch (sample) {
    if (control) {
      const sampleStack = sample.stack;
      if (sampleStack != null) {
        return [sampleStack, control.stack];
      }
    }
  }
}

export function controlStack() {
  try {
    throw new Error();
  } catch (x) {
    const control = x;
    return control?.stack;
  }
}

export function described(value) {
  try {
    return "" + value;
  } catch (x) {
    return `\nError describing: ${x.message}\n${x.stack ?? ""}`;
  }
}
```

- **`let Err(x) = catch_unwind(|| panic!(""));` is a `try`**, the
  pattern rustc asks for where the closure never returns, its `Ok`
  uninhabited, rather than an `if let` it warns is irrefutable. As nothing
  after it runs but once it threw, what follows it that reads `x`, or
  reads what that declares, is the `catch`, and the rest of the block
  follows the `try`: `controlStack`'s `return`, and `stacks`' `control =
  x;` before `render()` ([ADR 0830](../../decisions/0830-what-follows-a-let-err-of-a-closure-that-never-returns-is-its-catch.md)).
- **`match catch_unwind(f) { Ok(()) => return, Err(e) => .. }` is a
  `try` that returns**, the `Err` arm its `catch`, and what follows the
  `match` after the `try` ([ADR 0637](../../decisions/0637-a-match-of-a-throwing-call-is-try-catch.md#amendment-2026-10-25-an-ok-arm-that-leaves-what-follows-reading-the-err)).
  A `return` inside the closure, in a loop, is the function's own where
  the `try` ends the function, as nothing runs after it but its
  `finally` ([ADR 0660](../../decisions/0660-catch-unwind-catches-what-js-throws.md#amended-2026-10-25-a-return-in-the-closure-of-a-try-that-ends-the-function)).
- **What it unwinds through is dropped** before `catch_unwind` returns,
  as [above](#unwinding-runs-drop). The payload drops nothing, a panic's
  message or what JS threw, unless the crate makes a `dyn Any` of a value
  with a destructor, or a library it uses says it does, so a crate with
  destructors catches as JS's `try` too ([ADR 0691](../../decisions/0691-a-dyn-any-drops-where-the-crate-makes-one-that-does.md)). rustc checks `UnwindSafe`;
  `AssertUnwindSafe(|| ..)` is the closure it holds.
- **It never unwinds itself**: what's held around it isn't dropped in a
  `catch` of its call, so an `if let Err(_) = catch_unwind(f)` beside a
  value a drop flag tracks is a `try` too ([ADR 0873](../../decisions/0873-a-function-handed-to-js-carries-no-drop.md)).
- **A future's panic is its promise's**: `catch_unwind` of a closure that
  makes one is `Ok` of it, and the panic is thrown where it's awaited.
- **Refused**: `std::panic::set_hook` and `take_hook`, as rust-js has no
  hook, and `catch_unwind` in a crate whose panics abort, `panic =
  "abort"`, as rust-js's always unwind:

```text
error: rust-js does not support unwinding where a panic aborts, `panic = "abort"`: rust-js's panics unwind yet
```

Return a `Result` for a failure the caller handles.

## Throw a JS value: `js::throw`

`js::throw(value)` is JS's `throw value`: a value JS already has, thrown
as it is, an `Error` it gave you, a promise, a string, `undefined` of
`None` ([ADR 0760](../../decisions/0760-js-throw-is-throw-of-a-js-value.md)).
React's `lazyInitializer` throws what its loader rejected with, or the
thenable it's waiting on, which Suspense catches:

```rust
use std::cell::Cell;

use js::Unknown;

#[derive(Clone, Copy, PartialEq)]
pub enum Status {
    Pending,
    Resolved,
}

pub struct Payload {
    #[cfg_attr(rust_js, rust_js::name = "_status")]
    pub status: Cell<Status>,
    #[cfg_attr(rust_js, rust_js::name = "_result")]
    pub result: Cell<Option<&'static Unknown>>,
}

pub fn lazy_initializer(payload: &Payload) -> Option<&'static Unknown> {
    if payload.status.get() == Status::Resolved {
        payload.result.get()
    } else {
        js::throw(payload.result.get())
    }
}
```

```js
export function lazyInitializer(payload) {
  if (payload._status !== "Resolved") {
    throw payload._result;
  }
  return payload._result;
}
```

- **It's the one way to throw a JS value.** `panic!` is the one way to
  throw a new `Error`; `resume_unwind` of what `catch_unwind` caught
  throws it again, as it was thrown.
- **It returns `!`**, as `panic!` does, so it ends a branch of any type.
- **What it throws unwinds as any JS throw**: `catch_unwind` catches it,
  and its payload's `downcast_ref::<&JsError>()` is the value thrown, a
  string's or a promise's too, as [above](#catch-an-unwind-catch_unwind).

## Exiting the process

`std::process::exit(code)` is Node's `process.exit(code)`:

```rust
pub fn stop() {
    std::process::exit(2);
}
```

```js
import { $exit } from "@rust-js/runtime";

export function stop() {
  $exit(2);
}
```

In a browser, which has no process, it panics:

```text
`std::process::exit` needs Node's process, which a browser hasn't (ADR 0391)
```

`std::process::abort` is refused.
