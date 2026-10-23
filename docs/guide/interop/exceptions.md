# Errors, panics and exceptions

Rust has two kinds of failure, and each is what JS would make of it: a
`Result` is a value you return, and a panic is a thrown `Error`. A JS
exception reaches Rust as an `Err`, where you ask for one, or as a panic.

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
- [JS catches a panic; Rust can't](#js-catches-a-panic-rust-cant)
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
  const result = $try(() => loadConfig(path));
  return result.TAG === "Ok" ? result._0 : "";
}

export function config(path) {
  return $unwrapOk(
    $try(() => loadConfig(path)),
    "the config loads",
    String,
  );
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
- **`&JsError` is whatever was thrown**: `message()`, `name()`,
  `cause()` and `stack()` read an `Error`'s; `is_error()` says whether it
  is one, since JS may throw anything. Its `{:?}` is `String(error)`,
  `TypeError: no file missing`, what an `expect` shows.
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
  if (fail) {
    try {
      throw new Error("failed");
    } catch (error) {
      console.log(`closing ${name}`);
      console.log(`closing ${_outer.name}`);
      throw error;
    }
  }
  try {
    console.log(`closing ${name}`);
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
    const value = `${entries.value.length} entries`;
    try {
      borrowMut.value.push(value);
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

## JS catches a panic; Rust can't

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
exits with 101. Rust code can't catch a panic: `std::panic::catch_unwind`
is refused, and so is `std::panic::set_hook`:

```text
error: rust-js does not support calling `std::panic::catch_unwind` yet
```

Return a `Result` for a failure the caller handles.

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
