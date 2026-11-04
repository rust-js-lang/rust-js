# Advanced: closures, errors, async, traits

Closures, classes, errors, promises, and the part of Rust JavaScript
doesn't have: ownership. Read [Basics](basics.md) and
[Collections](collections.md) first. Each section goes: the JS you'd
write, the Rust, then rust-js's JS.

## Contents

- [Closures](#closures)
- [Classes](#classes)
- [Inheritance and interfaces: traits](#inheritance-and-interfaces-traits)
- [Generics](#generics)
- [Errors: `throw` and `try`](#errors-throw-and-try)
- [Promises and `async`](#promises-and-async)
- [Ownership and borrowing](#ownership-and-borrowing)
- [Shared mutable state: `Rc<RefCell<T>>`](#shared-mutable-state-rcrefcellt)
- [Iterators and generators](#iterators-and-generators)

## Closures

A closure is an arrow function. One that outlives the function that made
it, returned or stored, takes what it uses with `move`. One of statements
that a `let` binds is a function declared in its place, as JS writes a
helper inside a function; one of a value alone stays an arrow
([ADR 0882](../../decisions/0882-a-field-of-statements-is-a-method.md#amendment-a-lets-closure-of-statements-is-a-function-declared)),
`function counted(n) { .. }` beside `const double = (n) => n * 2`.

```js
const makeAdder = (n) => (x) => x + n;

function makeCounter() {
  let count = 0;
  return () => ++count;
}
```

```rust
pub fn make_adder(n: i32) -> impl Fn(i32) -> i32 {
    move |x| x + n
}

pub fn make_counter() -> impl FnMut() -> u32 {
    let mut count = 0;
    move || {
        count += 1;
        count
    }
}

pub fn apply<F: Fn(i32) -> i32>(f: F, x: i32) -> i32 {
    f(x)
}

pub type Callback = Box<dyn Fn(&str)>;

pub fn run_all(callbacks: &[Callback], msg: &str) {
    for cb in callbacks {
        cb(msg);
    }
}

pub fn tally(words: &[&str]) -> usize {
    let mut total = 0;
    words.iter().for_each(|w| total += w.len());
    total
}
```

```js
import { $byteLen } from "@rust-js/runtime";

export function makeAdder(n) {
  return (x) => (x + n) | 0;
}

export function makeCounter() {
  const count = 0;
  let count$1 = count;
  return () => {
    count$1 = (count$1 + 1) >>> 0;
    return count$1;
  };
}

export function apply(f, x) {
  return f(x);
}

export function runAll(callbacks, msg) {
  for (let i = 0; i < callbacks.length; i++) {
    const cb = callbacks[i];
    cb(msg);
  }
}

export function tally(words) {
  let total = 0;
  words.forEach((w) => {
    total = (total + $byteLen(w)) >>> 0;
  });
  return total;
}
```

- **The type says what a closure does with what it captures.** `Fn` only
  reads, `FnMut` changes, `FnOnce` uses it up and runs once. rustc picks
  the one your closure needs; you name it in a parameter or return type.
- `impl Fn(i32) -> i32` is "some closure of this shape". Take a closure
  as a parameter with a generic, `F: Fn(i32) -> i32`, or as
  `Box<dyn Fn(&str)>` to keep different closures in one `Vec`.
- `move` gives a closure its own copy of what it captures. Without it, a
  closure borrows, and rustc checks it doesn't outlive what it borrows.
  In JS, every closure captures by reference; with `move`, the moved
  variable is the closure's own, which is why `count` is copied to
  `count$1`.

A closure that only passes what it took by value, then each of its own
parameters, to a function of the crate's is that function bound to what
it took, `bind`'s partial application, as React writes
`startTransition.bind(null, fiber, queue, true, false)`
([ADR 0605](../../decisions/0605-an-arrow-that-passes-on-has-one-owner.md),
amended). `bind` reads what it binds where it's made, as the closure takes
it. So is one of no parameters of its own, `move || dispatch(&queue,
false, 0)`, returned, held or made an `Rc`'s, and one of rest arguments
it spreads on to a function's rest: alien-signals' `computed` is
`computedOper.bind(null, node)`, whose `name`, `"bound computedOper"`,
its `isComputed` reads. One of no parameters handed on as a callback,
given to a call, boxed or lent, `schedule_work(move ||
perform_work(&request))`, stays the arrow JS writes in place,
`scheduleWork(() => performWork(request))`, a hook's `useMemo(() =>
filterTodos(todos, tab), ..)` too. So does one that borrows what it
passes, reads a value when it's called, makes its call a statement,
`move || { dispatch(&queue, false, 0); }`, or calls a closure:

```rust
use std::rc::Rc;

pub struct Queue {
    pub name: String,
}

fn dispatch(queue: &Rc<Queue>, eager: bool, action: u32) {
    println!("{} {eager} {action}", queue.name);
}

pub fn dispatcher(queue: Rc<Queue>) -> Rc<dyn Fn(u32)> {
    Rc::new(move |action| dispatch(&queue, true, action))
}

pub fn later(queue: Rc<Queue>) -> Rc<dyn Fn()> {
    Rc::new(move || dispatch(&queue, false, 0))
}
```

```js
function dispatch(queue, eager, action) {
  console.log(`${queue.name} ${eager} ${action}`);
}

export function dispatcher(queue) {
  return dispatch.bind(null, queue, true);
}

export function later(queue) {
  return dispatch.bind(null, queue, false, 0);
}
```

A `let` of your own `Rc::new`, `Arc::new` or `Box::new` of a closure
holds a function as a value, `let hook: Rc<dyn Fn()> = Rc::new(..)`: a
`const` of its arrow, `const hook = () => { .. }`, as TypeScript writes a
value of a function's type; a binding's `Box`, an event's `spawn`'s, is
a function declared, `async function handleCopy() { .. }`
([ADR 0882](../../decisions/0882-a-field-of-statements-is-a-method.md#amendment-a-function-held-as-a-value-is-a-const-of-its-arrow)).

A closure that refers to itself is written as the Rust book teaches a
value that refers to itself: its `Rc`'s `Weak` in a cell it captured,
set right after it's made, and upgraded where it calls or names itself.
It runs only borrowed from its `Rc`, so the upgrade always gives it: where
nothing counts it, the cell is no JS and the upgrade is its own name
([ADR 0843](../../decisions/0843-a-weak-nothing-upgrades-is-the-value-itself.md#amendment-a-closures-handle-on-itself-is-its-own-name)).
One whose counts are read, or whose cell is read otherwise, is counted,
as written.

```rust
use std::cell::RefCell;
use std::rc::{Rc, Weak};

pub type Countdown = dyn Fn(u32) -> u32;

pub fn countdown(step: u32) -> Rc<Countdown> {
    let this: Rc<RefCell<Option<Weak<Countdown>>>> = Rc::new(RefCell::new(None));
    let countdown: Rc<Countdown> = Rc::new({
        let this = Rc::clone(&this);
        move |n| {
            if n < step {
                return n;
            }
            let countdown = this.borrow().as_ref().and_then(Weak::upgrade).unwrap();
            1 + countdown(n - step)
        }
    });
    *this.borrow_mut() = Some(Rc::downgrade(&countdown));
    countdown
}
```

```js
export function countdown(step) {
  const countdown = (n) => {
    if (n < step) {
      return n;
    }
    return (1 + countdown(n - step)) >>> 0;
  };
  return countdown;
}
```

Callbacks made before the value they need, given to the call that makes
it, as React's server entries make `onAllReady` before the request it
flows, share a cell of its `Weak`, set once right after the call
returns, so the value, which owns them, and they, which read it, make no
cycle. Where what the call returns alone owns them, each runs only while
the value is alive, and the cell is the value's `let`, which they close
over: `undefined`, `None`, until it's set, so a callback called before
reads `None`, and its `unwrap` panics as Rust's does
([ADR 0317](../../decisions/0317-once-cells.md#amendment-callbacks-handle-on-what-owns-them-is-its-let)).
Where nothing the call runs before it returns may call them, none runs
before the cell is set, so each `unwrap` is the value itself, as below:
no code it may run names a field they're kept in, and none is what
rust-js can't follow, a `dyn Fn`'s call or a drop of one that runs a
destructor. A library's contract says so of its function
([ADR 0317, amended](../../decisions/0317-once-cells.md#amendment-a-callback-nothing-calls-before-its-call-returns-reads-the-value-rd621-a)).
Otherwise the `unwrap` stays, `const request$1 = $unwrap(request);`.
One kept apart from the value, by a clone, a list or a field read but to
call it, or a cell set again or read otherwise, is counted, as written.
So is one kept in a public field, which any crate may clone out: keep a
library's callbacks in private fields, as above. Where a library whose
`dyn Fn`s drop nothing holds it, the crate is refused, and the refusal
names the callback, the cell and the call whose return doesn't own it
([ADR 0317, amended](../../decisions/0317-once-cells.md#amendment-a-callback-a-public-field-holds-isnt-the-returns-alone-rd553)).

```rust
use std::cell::OnceCell;
use std::rc::{Rc, Weak};

pub struct Request {
    pub name: String,
    on_all_ready: Rc<dyn Fn()>,
}

pub fn create_request(name: &str, on_all_ready: Rc<dyn Fn()>) -> Rc<Request> {
    Rc::new(Request {
        name: name.to_string(),
        on_all_ready,
    })
}

pub fn complete_all(request: &Request) {
    let on_all_ready = &request.on_all_ready;
    on_all_ready();
}

pub fn render(name: &str) -> Rc<Request> {
    let request_cell: Rc<OnceCell<Weak<Request>>> = Rc::new(OnceCell::new());
    let on_all_ready: Rc<dyn Fn()> = {
        let cell = Rc::clone(&request_cell);
        Rc::new(move || {
            let request = cell.get().and_then(Weak::upgrade).unwrap();
            println!("{} is ready", request.name);
        })
    };
    let request = create_request(name, on_all_ready);
    let _ = request_cell.set(Rc::downgrade(&request));
    request
}
```

```js
export function createRequest(name, onAllReady) {
  return {
    name,
    onAllReady,
  };
}

export function completeAll(request) {
  const onAllReady = request.onAllReady;
  onAllReady();
}

export function render(name) {
  let request;
  const onAllReady = () => {
    console.log(`${request.name} is ready`);
  };
  request = createRequest(name, onAllReady);
  return request;
}
```

## Classes

A class is a `struct` for its fields and an `impl` block for its methods.
A constructor is an ordinary associated function, by convention `new`.

```js
class Counter {
  constructor(step) {
    this.count = 0;
    this.step = step;
  }
  increment() {
    this.count += this.step;
  }
  get value() {
    return this.count;
  }
}
```

```rust
pub struct Counter {
    count: u32,
    step: u32,
}

impl Counter {
    pub fn new(step: u32) -> Self {
        Self { count: 0, step }
    }

    pub fn increment(&mut self) {
        self.count += self.step;
    }

    pub fn count(&self) -> u32 {
        self.count
    }

    pub fn reset(&mut self) {
        self.count = 0;
    }
}

impl Default for Counter {
    fn default() -> Self {
        Self::new(1)
    }
}

pub fn demo() -> u32 {
    let mut c = Counter::new(2);
    c.increment();
    c.increment();
    c.count()
}
```

```js
export const Counter = {
  new(step) {
    return { count: 0, step };
  },

  increment(counter) {
    counter.count = (counter.count + counter.step) >>> 0;
  },

  count(counter) {
    return counter.count;
  },

  reset(counter) {
    counter.count = 0;
  },
};

export const counterDefault = { default: counterDefault_default };
function counterDefault_default() {
  return Counter.new(1);
}

export function demo() {
  const c = Counter.new(2);
  Counter.increment(c);
  Counter.increment(c);
  return Counter.count(c);
}
```

- **A struct is a plain object, and its methods are an object of
  functions** that take it first: `c.increment()` is
  `Counter.increment(c)`. There's no `class`, no `this` and no prototype
  ([ADR 0047](../../decisions/0047-methods.md)). A private struct whose
  one value a function makes, and that nothing else holds, is that
  function's scope: its methods are functions declared in it, closing over
  the value
  ([ADR 1043](../../decisions/1043-a-functions-scope-struct-is-its-scope.md)).
- `&self` reads, `&mut self` changes, `self` takes the value.
- **A getter is a method,** `count(&self)`, called as `c.count()`. The
  field stays private; Rust has no `get` syntax. A setter is a method too,
  `set_step(&mut self, step: u32)`.
- `Default` is the constructor that takes no arguments,
  `Counter::default()`.
- A field without `pub` is private to its module.

How JavaScript that calls your crate sees a struct and its methods is in
[Structs, enums and traits as JavaScript sees them](../interop/objects.md).

## Inheritance and interfaces: traits

Rust has no `extends`. Shared behavior is a trait: an interface, whose
methods may have a default body, as a base class's would. Each type
`impl`s it.

```js
class Shape {
  describe() {
    return `a shape of area ${this.area().toFixed(1)}`;
  }
}
class Circle extends Shape {
  constructor(radius) { super(); this.radius = radius; }
  area() { return Math.PI * this.radius ** 2; }
}
```

```rust
use std::fmt;

pub trait Shape {
    fn area(&self) -> f64;

    fn describe(&self) -> String {
        format!("a shape of area {:.1}", self.area())
    }
}

pub struct Circle {
    pub radius: f64,
}

pub struct Square {
    pub side: f64,
}

impl Shape for Circle {
    fn area(&self) -> f64 {
        std::f64::consts::PI * self.radius * self.radius
    }
}

impl Shape for Square {
    fn area(&self) -> f64 {
        self.side * self.side
    }

    fn describe(&self) -> String {
        format!("a square of side {}", self.side)
    }
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Square({})", self.side)
    }
}

pub fn total_area(shapes: &[Box<dyn Shape>]) -> f64 {
    shapes.iter().map(|s| s.area()).sum()
}

pub fn largest<S: Shape>(a: S, b: S) -> S {
    if a.area() >= b.area() { a } else { b }
}

pub fn demo() -> String {
    let shapes: Vec<Box<dyn Shape>> = vec![
        Box::new(Circle { radius: 1.0 }),
        Box::new(Square { side: 2.0 }),
    ];
    let s = Square { side: 3.0 };
    format!("{} {} {}", total_area(&shapes), shapes[1].describe(), s)
}
```

```js
import { $displayF64, $index, $toFixed } from "@rust-js/runtime";

export const circleShape = {
  area: circleShape_area,
  describe: (self) => `a shape of area ${$toFixed(circleShape_area(self), 1)}`,
};
function circleShape_area(circle) {
  return Math.PI * circle.radius * circle.radius;
}

export const squareShape = { area: squareShape_area, describe: squareShape_describe };
function squareShape_area(square) {
  return square.side * square.side;
}

function squareShape_describe(square) {
  return `a square of side ${$displayF64(square.side)}`;
}

export const squareDisplay = { fmt: squareDisplay_fmt };
function squareDisplay_fmt(square) {
  return `Square(${$displayF64(square.side)})`;
}

export function totalArea(shapes) {
  return shapes.map((s) => s.impl.area(s.value)).reduce((a, b) => a + b, -0);
}

export function largest(a, b, SShape) {
  return SShape.area(a) >= SShape.area(b) ? a : b;
}

export function demo() {
  const shapes = [
    { value: { radius: 1 }, impl: circleShape },
    { value: { side: 2 }, impl: squareShape },
  ];
  const s = { side: 3 };
  const value = totalArea(shapes);
  const receiver = $index(shapes, 1);
  return `${$displayF64(value)} ${receiver.impl.describe(receiver.value)} ${squareDisplay_fmt(s)}`;
}
```

- **An `impl Trait for Type` is a dictionary,** an object of its
  functions, `circleShape`, its module's `const`.
- **A `Box<dyn Shape>`** is the value with its dictionary,
  `{ value, impl }`: a `Vec` of different shapes, as an array of
  subclasses would be.
- **A generic `S: Shape`** takes its dictionary as a last argument,
  `largest(a, b, circleShape)`. rustc checks every call passes a type
  that implements `Shape`.
- `Display` is the trait `{}` uses: implement it, and `format!("{s}")`
  and `s.to_string()` are your `toString()`.
- To share fields as well as methods, put the shared part in a struct and
  give each type a field of it.

([ADR 0049](../../decisions/0049-traits-and-generics.md).)

## Generics

A generic type or function works for any `T`, as in TypeScript. A bound,
`T: Display`, says what `T` must be able to do, and is checked at every
call.

```ts
class Stack<T> {
  items: T[] = [];
  push(item: T) { this.items.push(item); }
  peek(): T | undefined { return this.items.at(-1); }
}
```

```rust
use std::fmt::Display;

pub struct Stack<T> {
    items: Vec<T>,
}

impl<T> Stack<T> {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn push(&mut self, item: T) {
        self.items.push(item);
    }

    pub fn peek(&self) -> Option<&T> {
        self.items.last()
    }
}

impl<T> Default for Stack<T> {
    fn default() -> Self {
        Self::new()
    }
}

pub fn list<T: Display>(items: &[T]) -> String {
    items
        .iter()
        .map(|item| item.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn demo() -> String {
    let mut stack = Stack::new();
    stack.push(1);
    stack.push(2);
    format!("{:?} {}", stack.peek(), list(&["a", "b"]))
}
```

```js
export const Stack = {
  new() {
    return { items: [] };
  },

  push(stack, item) {
    stack.items.push(item);
  },

  peek(stack) {
    return stack.items.at(-1);
  },
};

export const stackDefault = { default: stackDefault_default };
function stackDefault_default() {
  return Stack.new();
}

export function list(items, TDisplay) {
  return items.map((item) => TDisplay.fmt(item)).join(", ");
}

export function demo() {
  const stack = Stack.new();
  Stack.push(stack, 1);
  Stack.push(stack, 2);
  const peek = Stack.peek(stack);
  return `${peek == null ? "None" : `Some(${peek})`} ${list(["a", "b"], { fmt: (value) => value })}`;
}
```

- A type parameter with no bound costs nothing: `push` is `push`.
- A bound is a dictionary argument, `TDisplay`, as for a trait above.
- An `Option<&T>` of a generic `T` is `$someAt`: `T` could itself be an
  `Option`, and `Some(None)` must stay apart from `None` ([ADR 0051](../../decisions/0051-generic-options.md)).

## Errors: `throw` and `try`

Rust has two kinds of error. One you expect, bad input, a missing file,
is a value: a function returns `Result<T, E>`, `Ok(value)` or `Err(error)`,
and `?` passes an `Err` up to the caller. One that's a bug is a panic:
`panic!`, `unwrap()` of `None`, an index past the end. A panic is a JS
`throw`.

```js
function port(line) {
  if (!line.startsWith("port=")) throw new Error("missing port");
  const port = Number(line.slice(5));
  if (!Number.isInteger(port)) throw new Error("bad number");
  return port;
}

function portOrDefault(line) {
  try {
    return port(line);
  } catch (e) {
    console.error(e.message);
    return 8080;
  }
}
```

```rust
use std::fmt;
use std::num::ParseIntError;

#[derive(Debug)]
pub enum ConfigError {
    Missing(String),
    BadNumber(ParseIntError),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConfigError::Missing(key) => write!(f, "missing {key}"),
            ConfigError::BadNumber(e) => write!(f, "bad number: {e}"),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<ParseIntError> for ConfigError {
    fn from(e: ParseIntError) -> Self {
        ConfigError::BadNumber(e)
    }
}

pub fn port(line: &str) -> Result<u16, ConfigError> {
    let value = line
        .strip_prefix("port=")
        .ok_or_else(|| ConfigError::Missing("port".to_string()))?;
    let port = value.parse()?;
    Ok(port)
}

pub fn port_or_default(line: &str) -> u16 {
    match port(line) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            8080
        }
    }
}

pub fn must_port(line: &str) -> u16 {
    port(line).expect("a valid port line")
}

pub fn ratio(a: u32, b: u32) -> u32 {
    if b == 0 {
        panic!("ratio of {a} to nothing");
    }
    a / b
}
```

```js
import {
  $debugParseError,
  $debugStr,
  $div,
  $parseInt,
  $stripPrefix,
  $unwrapOk,
} from "@rust-js/runtime";

export const configErrorDebug = { fmt: configErrorDebug_fmt };

function configErrorDebug_fmt(configError) {
  if (configError.TAG === "Missing") {
    return `Missing(${$debugStr(configError._0)})`;
  }
  return `BadNumber(${$debugParseError(configError._0, "ParseIntError")})`;
}

export const configErrorDisplay = { fmt: configErrorDisplay_fmt };
function configErrorDisplay_fmt(configError) {
  if (configError.TAG === "Missing") {
    return `missing ${configError._0}`;
  } else {
    return `bad number: ${configError._0}`;
  }
}

const $configErrorError_source = () => undefined;

export const configErrorError = {
  Debug: () => configErrorDebug,
  Display: () => configErrorDisplay,
  source: $configErrorError_source,
  description: () => "description() is deprecated; use Display",
  cause: $configErrorError_source,
};

function configErrorFromParseIntError_from(e) {
  return { TAG: "BadNumber", _0: e };
}

export function port(line) {
  const option = $stripPrefix(line, "port=");
  const result =
    option != null ? { TAG: "Ok", _0: option } : { TAG: "Err", _0: { TAG: "Missing", _0: "port" } };
  if (result.TAG === "Err") {
    return result;
  }
  const value = result._0;
  const result$1 = $parseInt(value, 0, 65535);
  if (result$1.TAG === "Err") {
    return { TAG: "Err", _0: configErrorFromParseIntError_from(result$1._0) };
  }
  const port = result$1._0;
  return { TAG: "Ok", _0: port };
}

export function portOrDefault(line) {
  const value = port(line);
  if (value.TAG === "Ok") {
    return value._0;
  } else {
    console.error(`${configErrorDisplay_fmt(value._0)}`);
    return 8080;
  }
}

export function mustPort(line) {
  return $unwrapOk(port(line), "a valid port line", configErrorDebug_fmt);
}

export function ratio(a, b) {
  if (b === 0) {
    throw new Error(`ratio of ${a} to nothing`);
  }
  return $div(a, b) >>> 0;
}
```

- **A `Result` is an object,** `{ TAG: "Ok", _0: 80 }` or
  `{ TAG: "Err", _0: error }`, and `?` is an early `return` of the `Err`.
  The signature says a function can fail; rustc won't let a caller ignore
  it.
- **`?` converts the error** through your `From` impl: a `ParseIntError`
  becomes a `ConfigError::BadNumber`.
- An error type is usually an `enum`, with `Display` for its message and
  `std::error::Error` so it works with `Box<dyn Error>`.
- **A panic is `throw new Error(message)`** with Rust's message:
  `expect` of an `Err` throws `a valid port line: Missing("port")`.
  Rust code can't catch a panic; use a `Result` for anything you want to
  handle. JavaScript that calls your crate can catch it.
- A quick program can return `Result<T, Box<dyn std::error::Error>>` and
  `?` any error into it.

How errors cross to and from JavaScript is in
[Errors, panics and exceptions](../interop/exceptions.md) ([ADR 0035](../../decisions/0035-results-and-throwing-js.md)).

## Promises and `async`

An `async fn` is an `async function`, `.await` is `await`, and an
`async move` block is an async arrow function. A JS caller gets a
`Promise`. The browser's `fetch` is in the `webapi` crate; JS's own
`Promise` functions, `spawn` and `settle` in the `js` crate.

```js
async function fetchText(url) {
  const response = await window.fetch(url);
  return await response.text();
}
```

```rust
use js::{settle, spawn};
use webapi::window;

pub async fn fetch_text(url: &str) -> String {
    let response = window.fetch(url, None).await;
    response.text().await
}

pub async fn try_fetch(url: &str) -> Result<String, String> {
    let response = settle(window.fetch(url, None))
        .await
        .map_err(|e| e.message())?;
    if !response.ok() {
        return Err(format!("HTTP {}", response.status()));
    }
    Ok(response.text().await)
}

pub fn on_click(url: &'static str) {
    spawn(Box::new(async move {
        let text = fetch_text(url).await;
        println!("{text}");
    }));
}
```

```js
import { $settle } from "@rust-js/runtime";

export async function fetchText(url) {
  const response = await window.fetch(url);
  return await response.text();
}

export async function tryFetch(url) {
  const result = await $settle(window.fetch(url));
  const result$1 = result.TAG === "Err" ? { TAG: "Err", _0: result._0.message } : result;
  if (result$1.TAG === "Err") {
    return result$1;
  }
  const response = result$1._0;
  if (!response.ok) {
    return { TAG: "Err", _0: `HTTP ${response.status}` };
  }
  return { TAG: "Ok", _0: await response.text() };
}

export function onClick(url) {
  (async () => {
    const text = await fetchText(url);
    console.log(`${text}`);
  })();
}
```

- **A rejected promise throws at its `.await`,** as in JS, and that's a
  panic in Rust. `settle(promise).await` makes the rejection an `Err` you
  handle: JS's `try`/`catch` around an `await`.
- **`spawn` runs a future no one waits for,** from an event handler say.
  A future is a JS promise, so it starts when it's made and runs to the
  end, where native Rust's waits to be polled.
- `Promise.all` is `js::promise::all`, of a `Vec` of promises.
  `js::promise(future)` turns an `async fn`'s future into the `Promise` it
  is:

```rust
use js::promise;
use webapi::window;

pub async fn fetch_text(url: &str) -> String {
    let response = window.fetch(url, None).await;
    response.text().await
}

pub async fn fetch_all(urls: &[&str]) -> Vec<String> {
    let pending = urls
        .iter()
        .map(|url| js::promise(fetch_text(url)))
        .collect();
    promise::all(pending).await
}
```

```js
export async function fetchText(url) {
  const response = await window.fetch(url);
  return await response.text();
}

export async function fetchAll(urls) {
  const pending = urls.map((url) => fetchText(url));
  return await Promise.all(pending);
}
```

More in [Promises and async](../interop/async.md) ([ADR 0029](../../decisions/0029-async-await.md), [ADR 0102](../../decisions/0102-js-and-webapi.md)).

## Ownership and borrowing

This is the part JavaScript doesn't have. Each value has one owner. A
function either takes it (`T`), borrows it to read (`&T`), or borrows it
to change (`&mut T`); rustc checks no one changes a value while someone
else reads it. In JS it's all the same object: ownership is checked at
compile time and costs nothing at run time.

```rust
#[derive(Debug, Clone)]
pub struct Settings {
    pub theme: String,
    pub size: u32,
}

pub fn copies() -> (Settings, Settings) {
    let a = Settings {
        theme: "dark".to_string(),
        size: 12,
    };
    let mut b = a.clone();
    b.size = 14;
    (a, b)
}

pub fn borrowed(s: &Settings) -> usize {
    s.theme.len()
}

pub fn grow(s: &mut Settings) {
    s.size += 1;
}
```

```js
import { $byteLen } from "@rust-js/runtime";

export function copies() {
  const a = {
    theme: "dark",
    size: 12,
  };
  const b = { ...a };
  b.size = 14;
  return [a, b];
}

export function borrowed(s) {
  return $byteLen(s.theme);
}

export function grow(s) {
  s.size = (s.size + 1) >>> 0;
}
```

- **`clone()` is a copy,** `{ ...a }`, where you'd write
  `structuredClone` or a spread in JS. rust-js copies only what needs it:
  a `String` is a JS string, which can't change, so it's shared.
- **`&T` and `&mut T` of an object are the object.** A move, `let b = a;`,
  is the same object too: rustc won't let you use `a` again, so nothing
  can tell.
- When rustc says a value was moved, or is borrowed, the fix is usually a
  `&` where you only read, or a `clone()` where you need two.

A `&mut` to a number or a string, which JS can't point to, is a box the
caller reads back:

```rust
pub fn bump(n: &mut u32) {
    *n += 1;
}

pub fn use_bump() -> u32 {
    let mut n = 1;
    bump(&mut n);
    n
}
```

```js
export function bump(n) {
  n.value = (n.value + 1) >>> 0;
}

export function useBump() {
  let n = 1;
  const n$1 = { value: n };
  bump(n$1);
  n = n$1.value;
  return n;
}
```

Returning the new value, `fn bumped(n: u32) -> u32`, is usually the
better Rust, and plain JS.

## Shared mutable state: `Rc<RefCell<T>>`

In JS, two objects can hold the same array and both push to it. In Rust,
a value shared by several owners is an `Rc<T>`, and one they also change
is an `Rc<RefCell<T>>`: `borrow()` to read it, `borrow_mut()` to change it.

```js
const lines = [];
const a = { lines };
const b = { lines };
a.lines.push("from a");
b.lines.push("from b");
```

```rust
use std::cell::RefCell;
use std::rc::Rc;

pub struct Logger {
    lines: Rc<RefCell<Vec<String>>>,
}

impl Logger {
    pub fn log(&self, line: &str) {
        self.lines.borrow_mut().push(line.to_string());
    }
}

pub fn two_loggers() -> usize {
    let lines = Rc::new(RefCell::new(Vec::new()));
    let a = Logger {
        lines: Rc::clone(&lines),
    };
    let b = Logger {
        lines: Rc::clone(&lines),
    };
    a.log("from a");
    b.log("from b");
    lines.borrow().len()
}
```

```js
export const Logger = {
  log(logger, line) {
    logger.lines.push(line);
  },
};

export function twoLoggers() {
  const lines = [];
  const a = { lines };
  const b = { lines };
  Logger.log(a, "from a");
  Logger.log(b, "from b");
  return lines.length;
}
```

- **An `Rc` is the value itself,** and `Rc::clone` shares it, as two JS
  variables share one object. A clone made to give a `move` closure its
  own, of a parameter, of a variable bound once and never set again, or
  of what the closure around it captured by value, is read by that name,
  `() => flush(queue)` of `let queue = Rc::clone(&queue); move ||
  flush(&queue)`; so is one a closure makes of
  a parameter it borrows that isn't `mut`, `self` too, which in a method is
  `this`: `let chunk = Rc::clone(self);` in a promise's executor, then
  `chunk.info` in its callback, is `this.info`
  ([ADR 0846](../../decisions/0846-a-parameters-clone-is-read-by-its-name.md),
  [0879](../../decisions/0879-an-if-and-a-closures-captures-read-as-written.md)).
  What a closure takes by value that its maker reads first, a variable
  whose JS is a read of something else, is named for the variable, `const
  n = parse._0;` of `Ok(n) => move |x| x + n`; a part of a variable, and
  a copy of one the maker changes after, are `const captured = ..`
  ([ADR 0879](../../decisions/0879-an-if-and-a-closures-captures-read-as-written.md), amended).
- **A `RefCell` of an array, a `Map`, a `Set` or an object is that
  object**, `lines` above; of anything else, a number or an `Option`, it's
  `{ value }`. It counts its borrows where one may meet another, as std's
  does, and checks none where none can, as `lines`' can't
  ([ADR 0863](../../decisions/0863-a-refcell-of-an-object-is-the-object.md)):
  a `borrow_mut()` while another borrow is alive panics with
  `RefCell already borrowed`, as in native Rust ([ADR 0328](../../decisions/0328-refcell-borrow-counts.md)).
  A guard lives as long as Rust keeps it: a let-else's, `let Some(next) =
  update.next.borrow().clone() else { break };`, only while its
  initializer runs, so that borrow meets no other (ADR 0863, amended).
- **An element of one is its place**, as a `&mut` to one is: what
  `borrow_mut()[index]` reads and writes is the array's element, its
  index checked as Rust checks it
  ([ADR 0897](../../decisions/0897-an-element-a-guard-reaches-is-its-place.md)):

```rust
use std::cell::RefCell;

pub struct Root {
    pub lanes: RefCell<Vec<u32>>,
}

pub fn entangle(root: &Root, index: usize, lane: u32) {
    root.lanes.borrow_mut()[index] |= lane;
}
```

```js
import { $at } from "@rust-js/runtime";

export function entangle(root, index, lane) {
  root.lanes[$at(root.lanes, index)] = (root.lanes[index] | lane) >>> 0;
}
```

- A `Cell<T>` is the same for a `Copy` value, a number say, with `get` and
  `set` and no borrows to count.
- **`Cell::from_mut(&mut v)` of an object is a cell that writes `v` in
  place**, `$cellOf(v)`, as a write through the `&mut` does. What its
  `replace`, `take` or `swap` moves out is a copy of it, `$apart(..)`,
  since `v` itself becomes the new value. A `&mut` of an object a generic
  function keeps past the call, `fn set_get<'a, T>(a: &'a mut T, v: T) ->
  &'a mut T`'s, is the same cell, `set_get($cellOf(v), w)`. Of a type
  parameter's `&mut T` in generic code, already a box, `Cell::from_mut` is
  that box, as `get_mut` of a `Cell<T>` is
  ([ADR 0454](../../decisions/0454-a-mut-given-as-a-type-parameter.md), amended):

```rust
use std::cell::Cell;

pub fn emptied(v: &mut Vec<u32>) -> Vec<u32> {
    Cell::from_mut(v).replace(Vec::new())
}
```

```js
import { $apart, $cellOf } from "@rust-js/runtime";

export function emptied(v) {
  const item = $cellOf(v);
  const old = $apart(item.value);
  item.value = [];
  return old;
}
```

- In a React component, state is a hook instead: [Hooks](../react/hooks.md).

## Iterators and generators

Where JS has a generator function, Rust has an iterator: a struct with a
`next` method that returns `Some(item)`, or `None` at the end. Every
iterator method, `take`, `map`, `filter`, then works on it.
`std::iter::from_fn` makes one from a closure.

```js
function* fibonacci() {
  let [current, next] = [0n, 1n];
  for (;;) {
    yield current;
    [current, next] = [next, current + next];
  }
}

const firstFibs = (n) => Array.from(fibonacci().take(n));
```

```rust
pub struct Fibonacci {
    current: u64,
    next: u64,
}

impl Iterator for Fibonacci {
    type Item = u64;

    fn next(&mut self) -> Option<u64> {
        let value = self.current;
        self.current = self.next;
        self.next += value;
        Some(value)
    }
}

pub fn fibonacci() -> Fibonacci {
    Fibonacci {
        current: 0,
        next: 1,
    }
}

pub fn first_fibs(n: usize) -> Vec<u64> {
    fibonacci().take(n).collect()
}

pub fn countdown(from: u32) -> impl Iterator<Item = u32> {
    let mut n = from;
    std::iter::from_fn(move || {
        if n == 0 {
            None
        } else {
            n -= 1;
            Some(n + 1)
        }
    })
}

pub fn evens(limit: u32) -> impl Iterator<Item = u32> {
    (0..limit).filter(|n| n % 2 == 0)
}
```

```js
import { $fromFn, $iterator, $lazyRange } from "@rust-js/runtime";

function fibonacciIterator_next(fibonacci) {
  const value = fibonacci.current;
  fibonacci.current = fibonacci.next;
  fibonacci.next = BigInt.asUintN(64, fibonacci.next + value);
  return value;
}

export function fibonacci() {
  return {
    current: 0n,
    next: 1n,
  };
}

export function firstFibs(n) {
  return Array.from($iterator(fibonacci(), fibonacciIterator_next).take(n));
}

export function countdown(from) {
  const n = from;
  let n$1 = n;
  return Iterator.from(
    $fromFn(() => {
      if (n$1 !== 0) {
        n$1--;
        return n$1 + 1;
      }
    }),
  );
}

export function evens(limit) {
  return $lazyRange(0, limit).filter((n) => n % 2 === 0);
}
```

- **An `impl Iterator` is a JS iterator,** with JS's iterator helpers,
  `take`, `filter`, `map`: a JS caller can `for ... of` it. Collected,
  `collect::<Vec<_>>()`, it's `Array.from(it)`, as JS writes it, never
  ES2025's `it.toArray()`
  ([ADR 0055](../../decisions/0055-iterator.md), amended).
- An iterator of your own struct is its `next` function, wrapped by
  `$iterator` where a chain uses it.
- A `u64` is a BigInt, so `fibonacci` doesn't lose digits past 2⁵³.
