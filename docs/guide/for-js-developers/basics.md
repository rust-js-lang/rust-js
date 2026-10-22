# Basics: values, functions, control flow

You know JavaScript. This page shows, for each thing you write every day in
JavaScript, the Rust you write instead, and the JavaScript rust-js writes
for it. Each section goes: the JS you'd write, the Rust, then rust-js's JS.

The Rust is ordinary Rust: what you'd write for a server or a CLI. Nothing
here is special to rust-js. Every example was compiled by rust-js as it is
today; the JS shown is what it writes, without its header comment and
source-map line.

Two rules hold on every page:

- **Names become camelCase.** `first_word` is `firstWord`, a field
  `in_stock` is `inStock` ([ADR 0573](../../decisions/0573-every-rust-name-is-camel-case-in-js.md)).
- **A `pub fn` is an `export function`.** A private one stays in its module.

Next: [Collections](collections.md), then [Advanced](advanced.md).

## Contents

- [`const` and `let`](#const-and-let)
- [Numbers](#numbers)
- [Strings and template literals](#strings-and-template-literals)
- [Functions and arrow functions](#functions-and-arrow-functions)
- [Default and optional parameters](#default-and-optional-parameters)
- [Objects and structs](#objects-and-structs)
- [Destructuring](#destructuring)
- [`if` as an expression](#if-as-an-expression)
- [`switch` and `match`](#switch-and-match)
- [Loops](#loops)
- [`null` and `undefined`: `Option`](#null-and-undefined-option)
- [Equality](#equality)
- [`typeof` checks](#typeof-checks)
- [Modules and imports](#modules-and-imports)

## `const` and `let`

JS has `const` for a binding you never reassign and `let` for one you do.
Rust has `let` and `let mut`.

```js
function total(prices, tax) {
  const rate = 1 + tax;
  let sum = 0;
  for (const price of prices) {
    sum += price * rate;
  }
  return sum;
}
```

```rust
pub fn total(prices: &[f64], tax: f64) -> f64 {
    let rate = 1.0 + tax;
    let mut sum = 0.0;
    for price in prices {
        sum += price * rate;
    }
    sum
}
```

```js
export function total(prices, tax) {
  const rate = 1 + tax;
  let sum = 0;
  for (const price of prices) {
    sum += price * rate;
  }
  return sum;
}
```

- `let` is `const`; `let mut` is `let`.
- The last expression of a block, with no `;`, is its value: `sum` is the
  `return`.
- A module-level `const` is a JS `const`, exported if it's `pub`:

```rust
pub const MAX_USERS: usize = 100;
const GREETING: &str = "Welcome";

pub fn banner(count: usize) -> String {
    let left = MAX_USERS - count;
    format!("{GREETING}! {left} seats left")
}
```

```js
export const MAX_USERS = 100;
const GREETING = "Welcome";

export function banner(count) {
  const left = (MAX_USERS - count) >>> 0;
  return `${GREETING}! ${left} seats left`;
}
```

Rust lets you shadow a name with a new `let`, `let input = input.trim();`.
JS can't, so the second one gets a suffix, `input$1`.

## Numbers

JS has one number type. Rust has one per size, and you pick: `f64` for a
JS-like number, an integer type where the value is a count or an index.

| Rust | JS | Kept in range by |
|---|---|---|
| `f64` | number | nothing: it is a JS number |
| `f32` | number | `Math.fround` after each operation |
| `i8` `i16` `i32` `u8` `u16` `u32` | number | `\| 0`, `>>> 0`, `& 255` |
| `usize` `isize` | number, 32 bits | as `u32`, `i32` |
| `i64` `u64` `i128` `u128` | BigInt, `5n` | `BigInt.asIntN`, `BigInt.asUintN` |

```rust
pub fn average(xs: &[f64]) -> f64 {
    xs.iter().sum::<f64>() / xs.len() as f64
}

pub fn next_id(id: u32) -> u32 {
    id + 1
}

pub fn score(hits: i32, bonus: i32) -> i32 {
    hits * 10 + bonus
}

pub fn half(n: i32) -> i32 {
    n / 2
}

pub fn big(n: u64) -> u64 {
    n * 1_000_000
}

pub fn percent(part: u32, whole: u32) -> f64 {
    f64::from(part) / f64::from(whole) * 100.0
}
```

```js
export function average(xs) {
  return xs.reduce((a, b) => a + b, -0) / xs.length;
}

export function nextId(id) {
  return (id + 1) >>> 0;
}

export function score(hits, bonus) {
  return (((hits * 10) | 0) + bonus) | 0;
}

export function half(n) {
  return (n / 2) | 0;
}

export function big(n) {
  return BigInt.asUintN(64, n * 1000000n);
}

export function percent(part, whole) {
  return (part / whole) * 100;
}
```

What to know:

- **Integer division truncates**, `(n / 2) | 0`, as Rust's does.
- **Overflow wraps,** as a Rust release build's does: a `u8` of 100 plus
  200 is 44. Where you want something else, say so with std's methods:

```rust
pub fn checked(a: u32, b: u32) -> Option<u32> {
    a.checked_add(b)
}

pub fn saturating(a: u8, b: u8) -> u8 {
    a.saturating_sub(b)
}

pub fn to_byte(f: f64) -> u8 {
    f as u8
}

pub fn parse_age(s: &str) -> Result<u8, std::num::ParseIntError> {
    s.trim().parse()
}
```

```js
import { $checked, $f64ToInt, $parseInt, $trim } from "@rust-js/runtime";

export function checked(a, b) {
  return $checked(a + b, 0, 4294967295);
}

export function saturating(a, b) {
  return Math.max(a - b, 0);
}

export function toByte(f) {
  return $f64ToInt(f, 0, 255);
}

export function parseAge(s) {
  return $parseInt($trim(s), 0, 255);
}
```

- **Casts are Rust's:** `300.7 as u8` is `255`, `NaN as u8` is `0`.
- **`parse` is Rust's grammar,** not `Number()`'s: `"42"` parses, `" 42"`
  is an `Err`, so trim first.
- **Dividing an integer by zero panics,** with Rust's message. An `f64`
  divided by zero is `Infinity`, as in JS.

The whole story, with each difference from native Rust, is in
[Semantics](../semantics.md) ([ADR 0011](../../decisions/0011-numbers.md), [ADR 0086](../../decisions/0086-64-bit-integers.md)).

## Strings and template literals

A `String` and a `&str` are both a JS string. `format!` is a template
literal.

```js
function label(name, count) {
  return `${name} has ${count} items`;
}

function price(cents) {
  return `$${Math.trunc(cents / 100)}.${String(cents % 100).padStart(2, "0")}`;
}
```

```rust
pub fn label(name: &str, count: u32) -> String {
    format!("{name} has {count} items")
}

pub fn price(cents: u32) -> String {
    format!("${}.{:02}", cents / 100, cents % 100)
}

pub fn shout(s: &str) -> String {
    s.to_uppercase() + "!"
}

pub fn log_user(name: &str, age: u8) {
    println!("user {name}, age {age}");
}
```

```js
export function label(name, count) {
  return `${name} has ${count} items`;
}

export function price(cents) {
  return `$${(cents / 100) >>> 0}.${String(cents % 100).padStart(2, "0")}`;
}

export function shout(s) {
  return s.toUpperCase() + "!";
}

export function logUser(name, age) {
  console.log(`user ${name}, age ${age}`);
}
```

- `{name}` names a variable in scope; `{}` takes the next argument.
- Width, fill and precision are Rust's: `{:02}`, `{:>5}`, `{:.1}`.
- `println!` is `console.log`, `eprintln!` is `console.error`.
- `{:?}` is Rust's debug form: a string in quotes, a struct with its
  name and fields. Use it for logging, not for UI text.

`s.len()` is the length in UTF-8 bytes, not `s.length`. See
[Collections: strings](collections.md#strings).

## Functions and arrow functions

A `fn` is a `function`; a closure, `|n| n * 2`, is an arrow function.
Parameter and return types are written out; inside a body, Rust infers.

```js
function add(a, b) {
  return a + b;
}

function applyTwice(x) {
  const double = (n) => n * 2;
  return double(double(x));
}
```

```rust
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub fn apply_twice(x: i32) -> i32 {
    let double = |n: i32| n * 2;
    double(double(x))
}
```

```js
export function add(a, b) {
  return (a + b) | 0;
}

export function applyTwice(x) {
  const double = (n) => (n * 2) | 0;
  return double(double(x));
}
```

Closures that capture and change their surroundings are in
[Advanced: closures](advanced.md#closures).

## Default and optional parameters

Rust has no default parameters. A parameter a caller may leave out is an
`Option`, and `unwrap_or` gives its default. A JS caller passes
`undefined`, or nothing.

```js
function greeting(name = "stranger") {
  return `Hello, ${name}`;
}

function repeat(text, times = 1) {
  return text.repeat(times);
}
```

```rust
pub fn greeting(name: Option<&str>) -> String {
    let name = name.unwrap_or("stranger");
    format!("Hello, {name}")
}

pub fn repeat(text: &str, times: Option<usize>) -> String {
    text.repeat(times.unwrap_or(1))
}
```

```js
export function greeting(name) {
  name ??= "stranger";
  return `Hello, ${name}`;
}

export function repeat(text, times) {
  return text.repeat(times ?? 1);
}
```

`None` is `undefined`, so `unwrap_or` is `??`. A Rust caller writes
`greeting(None)` or `greeting(Some("Ada"))`.

## Objects and structs

A JS object with a fixed set of fields is a Rust `struct`. Declare it once;
rustc checks every use. In JS it's a plain object literal.

```js
const origin = () => ({ x: 0, y: 0 });
const make = (x, y) => ({ x, y });
function shift(p, dy) {
  p.y += dy;
}
```

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

pub fn origin() -> Point {
    Point { x: 0.0, y: 0.0 }
}

pub fn make(x: f64, y: f64) -> Point {
    Point { x, y }
}

pub fn shift(p: &mut Point, dy: f64) {
    p.y += dy;
}

pub fn moved(p: &Point, dx: f64) -> Point {
    Point {
        x: p.x + dx,
        ..p.clone()
    }
}
```

```js
export function origin() {
  return { x: 0, y: 0 };
}

export function make(x, y) {
  return { x, y };
}

export function shift(p, dy) {
  p.y += dy;
}

export function moved(p, dx) {
  return { x: p.x + dx, y: { ...p }.y };
}
```

- `&mut Point` is the object itself, changed in place. `&Point` is the
  object too; rustc won't let you change it.
- `..p.clone()` is Rust's spread, `{ ...p, x }`; the rest of the fields
  come from `p`.
- Methods go in an `impl` block: [Advanced: classes](advanced.md#classes).
- An object used as a dictionary, with keys only known at run time, is a
  `HashMap`: [Collections: maps](collections.md#map-and-set).

How a struct looks to JavaScript that calls your crate is in
[Structs, enums and traits as JavaScript sees them](../interop/objects.md).

## Destructuring

Rust's `let` takes a pattern, as JS's destructuring does, and a function
parameter can be one too.

```js
function describe(user) {
  const { name, age } = user;
  return `${name} (${age})`;
}

function swap([a, b]) {
  return [b, a];
}
```

```rust
pub struct User {
    pub name: String,
    pub age: u32,
    pub email: Option<String>,
}

pub fn describe(user: &User) -> String {
    let User { name, age, .. } = user;
    format!("{name} ({age})")
}

pub fn contact(User { name, email, .. }: &User) -> String {
    match email {
        Some(email) => format!("{name} <{email}>"),
        None => name.clone(),
    }
}

pub fn swap(pair: (i32, i32)) -> (i32, i32) {
    let (a, b) = pair;
    (b, a)
}

pub fn first_two(items: &[String]) -> Option<String> {
    let [first, second, ..] = items else {
        return None;
    };
    Some(format!("{first} and {second}"))
}
```

```js
export function describe(user) {
  const { name, age } = user;
  return `${name} (${age})`;
}

export function contact({ name, email }) {
  if (email != null) {
    return `${name} <${email}>`;
  }
  return name;
}

export function swap(pair) {
  const [a, b] = pair;
  return [b, a];
}

export function firstTwo(items) {
  if (items.length < 2) {
    return;
  }
  const [first, second] = items;
  return `${first} and ${second}`;
}
```

- `..` skips the fields you don't name. Rust makes you say so.
- A tuple, `(i32, i32)`, is a JS array, `[a, b]`.
- `let ... else` handles the pattern not matching: here, fewer than two
  items.

## `if` as an expression

An `if` has a value, so it's JS's `?:` as well as its `if`.

```js
const rate = member ? 0.01 : 0.03;
```

```rust
pub fn sign(n: i32) -> &'static str {
    if n < 0 {
        "negative"
    } else if n == 0 {
        "zero"
    } else {
        "positive"
    }
}

pub fn fee(member: bool, amount: f64) -> f64 {
    let rate = if member { 0.01 } else { 0.03 };
    amount * rate
}
```

```js
export function sign(n) {
  if (n < 0) {
    return "negative";
  }
  return n === 0 ? "zero" : "positive";
}

export function fee(member, amount) {
  const rate = member ? 0.01 : 0.03;
  return amount * rate;
}
```

An `if` condition must be a `bool`. There's no truthiness: write
`!s.is_empty()`, `n != 0`, `opt.is_some()`.

## `switch` and `match`

`match` is `switch` that can't fall through, can't forget a case, and has a
value. rustc rejects a `match` that misses one.

```js
function dayName(day) {
  switch (day) {
    case 0: return "Sunday";
    case 6: return "Saturday";
    case 1: case 2: case 3: case 4: case 5: return "weekday";
    default: return "unknown";
  }
}
```

```rust
pub fn day_name(day: u8) -> &'static str {
    match day {
        0 => "Sunday",
        6 => "Saturday",
        1..=5 => "weekday",
        _ => "unknown",
    }
}

pub fn http_status(code: u16) -> String {
    match code {
        200 | 204 => "ok".to_string(),
        404 => "not found".to_string(),
        c if c >= 500 => format!("server error {c}"),
        c => format!("status {c}"),
    }
}
```

```js
export function dayName(day) {
  if (day === 0) {
    return "Sunday";
  }
  if (day === 6) {
    return "Saturday";
  }
  if (day >= 1) {
    if (day <= 5) {
      return "weekday";
    }
  }
  return "unknown";
}

export function httpStatus(code) {
  if (code === 200 || code === 204) {
    return "ok";
  }
  if (code === 404) {
    return "not found";
  }
  if (code >= 500) {
    return `server error ${code}`;
  }
  return `status ${code}`;
}
```

`|` is several cases in one arm, `1..=5` a range, and `if` after a pattern
a guard. A `match` is an `if` chain, never a `switch`
([ADR 0014](../../decisions/0014-match-lowering.md)). An arm that names a
`const` tests it by its name, and a comment before an arm is before its
body:

```rust
pub const IMMEDIATE: u8 = 1;
pub const IDLE: u8 = 5;

pub fn timeout(level: u8) -> f64 {
    match level {
        // Times out immediately
        IMMEDIATE => -1.0,
        // Never times out
        IDLE => 1073741823.0,
        _ => 5000.0,
    }
}
```

```js
export const IMMEDIATE = 1;
export const IDLE = 5;

export function timeout(level) {
  if (level === IMMEDIATE) {
    // Times out immediately
    return -1;
  }
  if (level === IDLE) {
    // Never times out
    return 1073741823;
  }
  return 5000;
}
```

Where JS uses string constants or a `kind` field to tell cases apart, Rust
uses an `enum`. An enum without fields is its variant's name as a string;
one with fields is an object with a `TAG`:

```js
const next = { Red: "Green", Yellow: "Red", Green: "Yellow" };

function area(shape) {
  switch (shape.TAG) {
    case "Circle": return Math.PI * shape.radius * shape.radius;
    case "Rect": return shape.width * shape.height;
  }
}
```

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Light {
    Red,
    Yellow,
    Green,
}

pub fn next(light: Light) -> Light {
    match light {
        Light::Red => Light::Green,
        Light::Yellow => Light::Red,
        Light::Green => Light::Yellow,
    }
}

pub enum Shape {
    Circle { radius: f64 },
    Rect { width: f64, height: f64 },
}

pub fn area(shape: &Shape) -> f64 {
    match shape {
        Shape::Circle { radius } => std::f64::consts::PI * radius * radius,
        Shape::Rect { width, height } => width * height,
    }
}

pub fn unit() -> Shape {
    Shape::Rect {
        width: 1.0,
        height: 1.0,
    }
}
```

```js
export function next(light) {
  if (light === "Red") {
    return "Green";
  }
  if (light === "Yellow") {
    return "Red";
  }
  return "Yellow";
}

export function area(shape) {
  if (shape.TAG === "Circle") {
    return 3.141592653589793 * shape.radius * shape.radius;
  }
  return shape.width * shape.height;
}

export function unit() {
  return { TAG: "Rect", width: 1, height: 1 };
}
```

A variant with unnamed fields, `Text(String)`, keeps them as `_0`, `_1`:
`{ TAG: "Text", _0: s }` ([ADR 0013](../../decisions/0013-fieldless-enums.md), [ADR 0033](../../decisions/0033-enums-with-fields.md)).

## Loops

`for ... of` is `for ... in`. `while` is `while`. `loop` is
`while (true)`, and its `break` can carry a value out.

```js
for (const name of names) console.log(name);
for (const [i, name] of names.entries()) console.log(`${i}: ${name}`);
```

```rust
pub fn print_all(names: &[String]) {
    for name in names {
        println!("{name}");
    }
}

pub fn numbered(names: &[String]) {
    for (i, name) in names.iter().enumerate() {
        println!("{i}: {name}");
    }
}

pub fn countdown(mut n: u32) {
    while n > 0 {
        println!("{n}");
        n -= 1;
    }
}

pub fn first_square_over(limit: u32) -> String {
    let mut n = 1;
    let square = loop {
        if n * n > limit {
            break n * n;
        }
        n += 1;
    };
    format!("{n} squared is {square}")
}

pub fn find_pair(grid: &[Vec<i32>], target: i32) -> Option<(usize, usize)> {
    for (r, row) in grid.iter().enumerate() {
        for (c, &cell) in row.iter().enumerate() {
            if cell == target {
                return Some((r, c));
            }
        }
    }
    None
}
```

```js
export function printAll(names) {
  for (const name of names) {
    console.log(`${name}`);
  }
}

export function numbered(names) {
  for (const [i, name] of names.entries()) {
    console.log(`${i}: ${name}`);
  }
}

export function countdown(n) {
  while (n > 0) {
    console.log(`${n}`);
    n -= 1;
  }
}

export function firstSquareOver(limit) {
  let n = 1;
  while (true) {
    if (Math.imul(n, n) >>> 0 > limit) {
      const square = Math.imul(n, n) >>> 0;
      return `${n} squared is ${square}`;
    }
    n = (n + 1) >>> 0;
  }
}

export function findPair(grid, target) {
  for (const [r, row] of grid.entries()) {
    for (const [c, cell] of row.entries()) {
      if (cell === target) {
        return [r, c];
      }
    }
  }
}
```

- A counting loop, `for (let i = 0; i < n; i++)`, is `for i in 0..n`; `0..=n`
  includes `n`.
- `break` and `continue` work as in JS, labels too: `'outer: for ...`,
  `break 'outer;`.

## `null` and `undefined`: `Option`

Rust has no `null`. A value that may be missing is an `Option<T>`: `Some(v)`
or `None`. In JS, `Some(v)` is `v` and `None` is `undefined`, so an
`Option` reads as the optional value you'd write by hand.

```js
const displayName = (p) => p.nickname ?? "anonymous";
const ageNextYear = (p) => (p.age != null ? p.age + 1 : undefined);
const initial = (p) => p.nickname?.[0];
```

```rust
pub struct Profile {
    pub nickname: Option<String>,
    pub age: Option<u32>,
}

pub fn display_name(p: &Profile) -> &str {
    p.nickname.as_deref().unwrap_or("anonymous")
}

pub fn age_next_year(p: &Profile) -> Option<u32> {
    p.age.map(|a| a + 1)
}

pub fn initial(p: &Profile) -> Option<char> {
    p.nickname.as_ref()?.chars().next()
}

pub fn is_adult(p: &Profile) -> bool {
    if let Some(age) = p.age {
        age >= 18
    } else {
        false
    }
}
```

```js
export function displayName(p) {
  return p.nickname ?? "anonymous";
}

export function ageNextYear(p) {
  return p.age != null ? (p.age + 1) >>> 0 : undefined;
}

export function initial(p) {
  if (p.nickname == null) {
    return;
  }
  return Array.from(p.nickname)[0];
}

export function isAdult(p) {
  return p.age != null && p.age >= 18;
}
```

- `unwrap_or` is `??`. `map` changes the value if there is one. `?` after
  an `Option` returns `None` from the function early, as `?.` stops a
  chain.
- `if let Some(age) = p.age` is "if it's there, call it `age`".
- rustc won't let you use an `Option<u32>` as a `u32`: you can't forget the
  missing case.

A value JS gives you that may be `null` is an `Option` too:
[JavaScript types](../interop/types.md).

## Equality

`==` in Rust is always a comparison by value, and both sides have the same
type. There's no `===` to remember.

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

pub fn same_name(a: &str, b: &str) -> bool {
    a == b
}

pub fn same_point(a: &Point, b: &Point) -> bool {
    a == b
}

pub fn same_list(a: &[i32], b: &[i32]) -> bool {
    a == b
}

pub fn same_ref(a: &Point, b: &Point) -> bool {
    std::ptr::eq(a, b)
}
```

```js
import { $eq, $ptrTo } from "@rust-js/runtime";

export function sameName(a, b) {
  return a === b;
}

export function samePoint(a, b) {
  return $eq(a, b);
}

export function sameList(a, b) {
  return $eq(a, b);
}

export function sameRef(a, b) {
  return $ptrTo(a) === $ptrTo(b);
}
```

- Numbers, strings and `bool`s compare with `===`.
- A struct compares field by field, an array item by item, where JS's
  `===` would ask whether they're the same object. `#[derive(PartialEq)]`
  is what lets you write `==` for your struct.
- "Is it the same object?" is `std::ptr::eq` ([ADR 0582](../../decisions/0582-ptr-eq-of-an-object-is-its-identity.md)).

## `typeof` checks

In your own code you don't check types at run time: rustc knows each one.
Where a value can be one of several kinds, it's an `enum`, and `match`
tells them apart:

```rust
pub enum Value {
    Number(f64),
    Text(String),
    Flag(bool),
}

pub fn describe(v: &Value) -> String {
    match v {
        Value::Number(n) => format!("number {n}"),
        Value::Text(s) => format!("string {s:?}"),
        Value::Flag(b) => format!("boolean {b}"),
    }
}
```

```js
import { $debugStr, $displayF64 } from "@rust-js/runtime";

export function describe(v) {
  if (v.TAG === "Number") {
    return `number ${$displayF64(v._0)}`;
  }
  if (v.TAG === "Text") {
    return `string ${$debugStr(v._0)}`;
  }
  return `boolean ${v._0}`;
}
```

`$displayF64` prints an `f64` as Rust does, which differs from JS's
`String(n)` for very large and very small numbers.

A value from JavaScript whose type you don't know, what an `any` API
returns, is a `js::Unknown`, and `js::classify` is `typeof`:

```rust
use js::{Kind, Unknown, classify};

pub fn describe(value: &Unknown) -> String {
    match classify(value) {
        Kind::String(s) => format!("text {s}"),
        Kind::Number(n) => format!("number {n}"),
        Kind::Bool(b) => format!("flag {b}"),
        Kind::Array(items) => format!("{} items", items.len()),
        _ => "something else".to_string(),
    }
}
```

```js
import { $displayF64 } from "@rust-js/runtime";

export function describe(value) {
  if (typeof value === "string") {
    return `text ${value}`;
  }
  if (typeof value === "number") {
    return `number ${$displayF64(value)}`;
  }
  if (typeof value === "boolean") {
    return `flag ${value}`;
  }
  if (Array.isArray(value)) {
    return `${value.length} items`;
  }
  return "something else";
}
```

The `js` crate is in [the crates rust-js ships](../libraries/README.md)
([ADR 0225](../../decisions/0225-unknown-values.md)).

## Modules and imports

Each Rust module is an ES module: `mod money;` in `lib.rs` is
`money.js` beside `lib.js`, and `use` is `import`.

```rust
// lib.rs
mod money;

use money::format_price;

pub fn receipt(item: &str, cents: u32) -> String {
    format!("{item}: {}", format_price(cents))
}
```

```rust
// money.rs
pub fn format_price(cents: u32) -> String {
    format!("${}.{:02}", cents / 100, cents % 100)
}
```

```js
// lib.js
import { formatPrice } from "./money.js";

export function receipt(item, cents) {
  const arg = formatPrice(cents);
  return `${item}: ${arg}`;
}
```

```js
// money.js
export function formatPrice(cents) {
  return `$${(cents / 100) >>> 0}.${String(cents % 100).padStart(2, "0")}`;
}
```

- `pub` is what other modules may use. A private item another module of
  yours uses is exported from its JS file anyway, as JS has no other way to
  share it.
- `mod util { ... }` written inline gets a file too, `util.js`.
- The runtime helpers, the `$` names, come from `@rust-js/runtime`.

How files are laid out is in [Emitting modules](../project/modules.md).
Importing a JavaScript or npm module is
[Write bindings for JavaScript APIs](../interop/bindings.md).
