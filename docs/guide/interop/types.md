# JavaScript types: what each Rust type is in JS

Every Rust value has one JavaScript form. It's the form a JavaScript
developer would pick: a struct is an object, a `Vec` an array, `None`
`undefined`. This page lists each one, so you know what a JS caller gets
from your crate and what a binding must be given.

## The table

| Rust | JavaScript | |
|---|---|---|
| `u8` `u16` `u32` `i8` `i16` `i32` | number, wrapped after each operation | [Numbers](#numbers) |
| `usize` `isize` | number, 32 bits | [Numbers](#numbers) |
| `u64` `i64` `u128` `i128` | BigInt, `5n` | [Numbers](#numbers) |
| `f64`, `f32` | number (`f32` rounded by `Math.fround`) | [Numbers](#numbers) |
| `bool` | boolean | |
| `char` | a string of one character | [Text](#text) |
| `String`, `&str`, `Box<str>` | string | [Text](#text) |
| `()` | `undefined` | |
| `Vec<T>`, `[T; N]`, `&[T]`, `VecDeque<T>` | array | [Collections](#collections) |
| tuple `(A, B)` | array `[a, b]` | [Collections](#collections) |
| `HashMap`, `BTreeMap` | `Map` | [Collections](#collections) |
| `HashSet`, `BTreeSet` | `Set` | [Collections](#collections) |
| struct with named fields | object `{ x, y }` | [Structs](#structs) |
| tuple struct | array `[a, b]` | [Structs](#structs) |
| unit struct | `undefined`, or its `#[rust_js::name]` string | [Structs](#structs) |
| `#[repr(transparent)]` struct | its field | [Structs](#structs) |
| fieldless enum variant | its name, `"Red"`; under a `#[repr(u8)]`, its number, its module's `const`, `Empty` | [Enums](#enums) |
| enum variant with fields | `{ TAG: "Circle", radius }`, `{ TAG: "Square", _0 }` | [Enums](#enums) |
| `#[rust_js::tag = "status"]` enum | `{ status: "done", value }` | [Enums](#enums) |
| `#[rust_js::untagged]` enum | its payload itself | [Enums](#enums) |
| `Option<T>` | the value, or `undefined` | [Option and Result](#option-and-result) |
| `#[rust_js::nullable]` `Option<T>` | the value, or `null` | [Option and Result](#option-and-result) |
| `#[rust_js::nullable]` `#[rust_js::optional]` field | absent until it's set, then the value or `null` | [Option and Result](#option-and-result) |
| `Result<T, E>` | `{ TAG: "Ok", _0 }`, `{ TAG: "Err", _0 }` | [Option and Result](#option-and-result) |
| `Box<T>`, `Rc<T>`, `Arc<T>` | the value itself | [Pointers](#box-rc-and-arc) |
| an `Rc` whose count is read | `{ value, strong, weak }` | [Pointers](#box-rc-and-arc) |
| `Cell<T>`, `RefCell<T>` | `{ value }`; in a field, the property itself; a `RefCell` of an array, a `Map`, a `Set` or an object, that object | [Cells](#cell-and-refcell) |
| `OnceCell<T>`, `OnceLock<T>` | `{ value }` of an `Option`; in a field, the property, absent until it's set | [OnceCell](#oncecell-and-oncelock) |
| `&T` | the value | [References](#references) |
| `&mut` to an object | the object, changed in place | [References](#references) |
| `&mut` to a number, a string, an `Option` | a box `{ value }` | [References](#references) |
| closure, `fn` pointer, `dyn Fn` | function | [Functions](#functions-and-trait-objects) |
| `dyn Trait` | `{ value, impl }` | [Functions](#functions-and-trait-objects) |
| a JS type, `struct X(PhantomData<JsObject>)` | the JS object | [JS values](#js-values-jsobject-and-unknown) |
| `js::Unknown` | any JS value | [JS values](#js-values-jsobject-and-unknown) |
| `js::SymbolFor` | the registry's symbol, `Symbol.for(key)` | [Symbols](#symbols) |

Names follow one rule too: [every Rust name is camelCase in JS](#names).

## Numbers

```rust
pub fn add(a: u32, b: u32) -> u32 {
    a + b
}

pub fn scale(a: i32) -> i32 {
    a * -3
}

pub fn next_byte(a: u8) -> u8 {
    a + 1
}

pub fn index_after(i: usize) -> usize {
    i + 1
}

pub fn micros(ms: u64) -> u64 {
    ms * 1000
}

pub fn half(x: f32) -> f32 {
    x / 2.0
}
```

```js
export function add(a, b) {
  return (a + b) >>> 0;
}

export function scale(a) {
  return (a * -3) | 0;
}

export function nextByte(a) {
  return (a + 1) & 255;
}

export function indexAfter(i) {
  return (i + 1) >>> 0;
}

export function micros(ms) {
  return BigInt.asUintN(64, ms * 1000n);
}

export function half(x) {
  return Math.fround(x / 2);
}
```

- **An integer of 32 bits or fewer is a number**, put back in its range
  after each operation: `>>> 0` for `u32`, `| 0` for `i32`, `& 255` for
  `u8`, `Math.imul` for a multiplication. Overflow wraps, as in a release
  build ([ADR 0011](../../decisions/0011-numbers.md)).
- **`usize` and `isize` are 32 bits**, as on `wasm32`: `usize::MAX` is
  `4294967295` ([ADR 0090](../../decisions/0090-wasm32-front-end.md)).
- **A 64-bit or 128-bit integer is a BigInt**, exact, wrapped with
  `BigInt.asUintN` or `BigInt.asIntN`. A JS caller passes `5n`, not `5`
  ([ADR 0086](../../decisions/0086-64-bit-integers.md)).
- **`f64` is a number; `f32` is a number rounded to 32 bits** after each
  operation.

A number's methods are JS's `Math` where they answer as Rust's do:
`x.sqrt()` is `Math.sqrt(x)`. Such a function taken as a `fn`, and a
constant of `std::f64::consts` that `Math` has, is `Math`'s itself
([ADR 0887](../../decisions/0887-std-items-that-are-math-are-math.md)):

```rust
use std::f64::consts::PI;

pub const LOG: fn(f64) -> f64 = f64::ln;

pub fn area(radius: f64) -> f64 {
    PI * radius * radius
}

pub fn rounding() -> fn(f64) -> f64 {
    f64::round
}
```

```js
import { $round } from "@rust-js/runtime";

export const LOG = Math.log;

export function area(radius) {
  return Math.PI * radius * radius;
}

export function rounding() {
  return (n) => $round(n);
}
```

- **Only where `Math`'s answers Rust's for every value of the type**:
  `u32::leading_zeros` and `i32::leading_zeros` are `Math.clz32`, an
  `f64`'s `floor`, `abs`, `sqrt`, `sin`, `atan2` and the rest of its
  `Math` calls are those functions, and so are an `f32`'s `floor`, `ceil`,
  `trunc` and `abs`, which are exact; `i32::unsigned_abs` is `Math.abs`
  and `i32::signum` `Math.sign`. `f64::round` takes a half away from zero,
  where `Math.round` takes it up, and an `f32`'s `sqrt` is rounded to an
  `f32`: each is the arrow of its call. `js::math` has `Math`'s own
  answers ([the js crate](../libraries/README.md#js-the-javascript-language)).
- **The constants are `E`, `LN_2`, `LN_10`, `LOG2_E`, `LOG10_E`, `PI`,
  `SQRT_2` and `FRAC_1_SQRT_2`**, `Math.E` to `Math.SQRT1_2`, the same
  numbers. A `const` written as one is it, `const LN2: f64 = LN_2` being
  `const LN2 = Math.LN2`; one written as a number, or computed, `PI / 2.0`,
  is the number.
- **A function item handed to an iterator is still its arrow**,
  `.map(f64::sqrt)` being `.map((n) => Math.sqrt(n))`, as a binding's is
  ([Bindings](bindings.md#a-binding-as-a-value)).

## Text

A `String`, a `&str` and a `Box<str>` are a JS string. A `char` is a string
of one character.

```rust
pub fn byte_len(text: &str) -> usize {
    text.len()
}

pub fn initial(name: &str) -> Option<char> {
    name.chars().next()
}
```

```js
import { $byteLen } from "@rust-js/runtime";

export function byteLen(text) {
  return $byteLen(text);
}

export function initial(name) {
  return Array.from(name)[0];
}
```

`len()`, slices and offsets count UTF-8 bytes, as Rust's do: `"héllo".len()`
is `6`, so it's `$byteLen(text)`, not `text.length`
([ADR 0138](../../decisions/0138-string-byte-counts.md)). `chars()` is by
code point.

## Collections

```rust
use std::collections::{BTreeMap, HashMap, HashSet};

pub fn scores() -> Vec<(String, u32)> {
    vec![("Ada".to_string(), 3)]
}

pub fn ages() -> HashMap<String, u32> {
    let mut ages = HashMap::new();
    ages.insert("Ada".to_string(), 36);
    ages
}

pub fn tags() -> HashSet<&'static str> {
    HashSet::from(["rust", "js"])
}

pub fn log_tags(tags: &HashSet<&str>) {
    tags.iter().for_each(|tag| println!("{tag}"));
}

pub fn ranks() -> BTreeMap<u32, &'static str> {
    BTreeMap::from([(2, "silver"), (1, "gold")])
}

pub fn ranked(ranks: &BTreeMap<u32, &'static str>) -> Vec<u32> {
    ranks.keys().copied().collect()
}

pub fn middle(items: &[i32]) -> &[i32] {
    &items[1..items.len() - 1]
}
```

```js
import { $cmp, $slice, $sortedEntries } from "@rust-js/runtime";

export function scores() {
  return [["Ada", 3]];
}

export function ages() {
  const ages = new Map();
  ages.set("Ada", 36);
  return ages;
}

export function tags() {
  return new Set(["rust", "js"]);
}

export function logTags(tags) {
  tags.forEach((tag) => {
    console.log(`${tag}`);
  });
}

export function ranks() {
  return new Map([
    [2, "silver"],
    [1, "gold"],
  ]);
}

export function ranked(ranks) {
  return $sortedEntries(ranks, $cmp).map((entry) => entry[0]);
}

export function middle(items) {
  return $slice(items, 1, (items.length - 1) >>> 0);
}
```

- **A `Vec`, an array, a slice and a `VecDeque` are a JS array.** A tuple is
  an array too.
- **A `HashMap` is a `Map` and a `HashSet` a `Set`.** They iterate in
  insertion order, where Rust's order is unspecified
  ([ADR 0059](../../decisions/0059-hashmap.md)). A key that's a struct, a
  tuple or an enum with a derived `Eq` is compared by value, in the
  runtime's `$KeyMap`. A set's `iter().for_each(..)` is the set's own
  `forEach`, `tags.forEach((tag) => ..)`, each item given in the order
  `Array.from(tags)` has them
  ([ADR 0877](../../decisions/0877-a-sets-for-each-is-its-own.md)).
- **A `BTreeMap` is a `Map` and a `BTreeSet` a `Set`,** iterated in key
  order.
- **A `js::Map` is JS's `Map` of any key**, as TypeScript's `Map<K, V>`
  is, each key the one JS's `SameValueZero` says, an object by itself,
  `undefined` and `null` apart, `NaN` itself: `Map<Option<&Unknown>, V>`
  keys a value of any shape, which no `HashMap` can
  ([ADR 0849](../../decisions/0849-a-map-takes-any-key.md)). One of what
  a `WeakMap` keys is a `WeakMap` too, as TypeScript's `Map` is: a
  `WeakMap` where the host has one, and a `Map` where it hasn't, is one
  value ([ADR 0699](../../decisions/0699-a-map-of-objects-is-a-weak-map.md)):

```rust
use js::{JsObject, Unknown, WeakMap};

thread_local! {
    pub static CACHE: &'static WeakMap<dyn Fn() -> Option<&'static Unknown>, String> =
        if js::is_defined(js::weak_map::new::<JsObject, String>) {
            js::weak_map::new()
        } else {
            js::map::new(None)
        };
}
```

```js
export const cache = typeof WeakMap !== "undefined" ? new WeakMap() : new Map();
```

- **A `js::Set` is JS's `Set` of any value**, as TypeScript's `Set<T>`
  is, each the one `SameValueZero` says, as a `js::Map`'s key. A set of
  what Rust hashes, the program's own types by value or by themselves, is
  a `HashSet`, a JS `Set` too. Each is made of what it starts with,
  `js::set::new(Some(&[frozen]))` `new Set([frozen])` and
  `js::map::new(Some(&[(key, value)]))` `new Map([[key, value]])`, or of
  nothing, `None`, as TypeScript's constructors take an optional array
  ([ADR 0875](../../decisions/0875-a-js-set-holds-any-value.md)):

```rust
use js::{Dict, Map, Set};

pub fn takes_frozen_keys() -> bool {
    let frozen = js::object::freeze(js::dict::new::<u32>());
    let map: &Map<Dict<u32>, bool> = js::map::new(Some(&[(frozen, true)]));
    let set: &Set<Dict<u32>> = js::set::new(Some(&[frozen]));
    map.has(frozen) && set.has(frozen)
}
```

```js
export function takesFrozenKeys() {
  const frozen = Object.freeze({});
  const map = new Map([[frozen, true]]);
  const set = new Set([frozen]);
  return map.has(frozen) && set.has(frozen);
}
```

- **A `js::Map`'s or a `js::WeakMap`'s value is any value**, as
  TypeScript's `Map<K, V>` takes any `V` and a `js::Dict`'s is. An
  `Option`'s `None` is `undefined`, as it is everywhere, and `get` tells
  a key whose value is `None` from a key the map lacks, `Some(None)` from
  `None`, as `map.has(key)` does: `$someGet(map, key)`, as a `HashMap`'s
  `get` of an `Option` is. JS's `null` is `js::Kind::Null`
  ([ADR 0884](../../decisions/0884-a-maps-value-is-any-value.md)):

```rust
use js::{Kind, Map};

pub fn has_bad_map_polyfill() -> bool {
    let frozen_object = js::object::freeze(js::dict::new::<Kind>());
    js::map::new(Some(&[(frozen_object, Kind::Null)]));
    js::set::new(Some(&[frozen_object]));
    false
}

pub fn counted(counts: &Map<str, Option<u32>>, key: &str) -> &'static str {
    match counts.get(key) {
        Some(Some(_)) => "counted",
        Some(None) => "not counted yet",
        None => "never seen",
    }
}
```

```js
import { $someGet, $someValue } from "@rust-js/runtime";

export function hasBadMapPolyfill() {
  const frozenObject = Object.freeze({});
  new Map([[frozenObject, null]]);
  new Set([frozenObject]);
  return false;
}

export function counted(counts, key) {
  const get = $someGet(counts, key);
  if (get != null) {
    if ($someValue(get) != null) {
      return "counted";
    }
    return "not counted yet";
  }
  return "never seen";
}
```

## Structs

```rust
pub struct Point {
    pub x: f64,
    pub y: f64,
}

pub struct Pair(pub u32, pub String);

pub struct Empty;

#[cfg_attr(rust_js, rust_js::name = "click")]
pub struct Click;

#[repr(transparent)]
pub struct UserId(pub u32);

pub fn point() -> Point {
    Point { x: 1.0, y: 2.0 }
}

pub fn pair() -> Pair {
    Pair(7, "seven".to_string())
}

pub fn empty() -> Empty {
    Empty
}

pub fn click() -> Click {
    Click
}

pub fn user(id: u32) -> UserId {
    UserId(id)
}

pub fn raw(user: &UserId) -> u32 {
    user.0
}
```

```js
export function point() {
  return { x: 1, y: 2 };
}

export function pair() {
  return [7, "seven"];
}

export function empty() {}

export function click() {
  return "click";
}

export function user(id) {
  return id;
}

export function raw(user) {
  return user;
}
```

- **A struct with named fields is an object**, its fields camelCase
  ([ADR 0020](../../decisions/0020-structs-and-tuples.md)).
- **A tuple struct is an array**, `[a, b]`, `.0` being `[0]`.
- **A unit struct is `undefined`**, as `()` is. One named
  `#[rust_js::name = "click"]` is that string: webapi's event names are
  ([ADR 0223](../../decisions/0223-webapi-event-and-tag-maps.md)).
- **A `#[repr(transparent)]` struct is its field.** `UserId(id)` is `id`, and
  `user.0` is `user`. Rust lays it out as its field, so JS has it as its
  field ([ADR 0584](../../decisions/0584-a-transparent-struct-is-its-field.md)).
  A newtype without the attribute is an array of one, `[id]`.

[Structs, enums and traits as JavaScript sees them](objects.md) covers
methods, `impl` blocks and copies.

## Enums

```rust
pub enum Color {
    Red,
    #[cfg_attr(rust_js, rust_js::name = "dark-green")]
    DarkGreen,
}

pub enum Shape {
    Circle { radius: f64 },
    Square(f64),
    Empty,
}

pub fn shapes() -> Vec<Shape> {
    vec![Shape::Circle { radius: 1.0 }, Shape::Square(2.0), Shape::Empty]
}

pub fn area(shape: &Shape) -> f64 {
    match shape {
        Shape::Circle { radius } => std::f64::consts::PI * radius * radius,
        Shape::Square(side) => side * side,
        Shape::Empty => 0.0,
    }
}
```

```js
export function shapes() {
  return [{ TAG: "Circle", radius: 1 }, { TAG: "Square", _0: 2 }, "Empty"];
}

export function area(shape) {
  if (shape.TAG === "Circle") {
    return Math.PI * shape.radius * shape.radius;
  }
  if (shape.TAG === "Square") {
    return shape._0 * shape._0;
  }
  return 0;
}
```

- **A variant without fields is its name**, `"Red"`, or its
  `#[rust_js::name]`, `"dark-green"`
  ([ADR 0013](../../decisions/0013-fieldless-enums.md)).
- **A variant of an enum under a `#[repr(..)]` of an integer is its
  number**, as rustc numbers it, `ArrayKind::Empty` `0`, where JS keeps a
  kind as a number, React's `const Pending = 0;`: each variant is its
  module's `const`, named as the variant is, `const Empty = 0;`, and read
  by that name, imported where another module or crate reads it, as a
  `const` is. Its `as u8` is the value, its `{:?}` its name, its
  `.d.ts` `0 | 1 | 2 | 3` and `export const Empty: 0;`, and serde still
  writes its name. A variant's `#[rust_js::name]` names its `const`,
  React's `const UNTERMINATED = 0;`, a JS name or refused. A 64-bit
  `repr` keeps the names
  ([ADR 0722](../../decisions/0722-a-repr-enums-variant-is-its-number.md)):

```rust
#[derive(Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum ArrayKind {
    Empty = 0,
    Complex = 1,
}

pub fn kind(complex: bool) -> ArrayKind {
    if complex { ArrayKind::Complex } else { ArrayKind::Empty }
}
```

```js
export const Empty = 0;
export const Complex = 1;

export function kind(complex) {
  return complex ? Complex : Empty;
}
```

Named, its `const` is that name:

```rust
#[derive(Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Status {
    #[cfg_attr(rust_js, rust_js::name = "UNTERMINATED")]
    Unterminated = 0,
    #[cfg_attr(rust_js, rust_js::name = "TERMINATED")]
    Terminated = 1,
}

pub fn done(status: Status) -> bool {
    status == Status::Terminated
}
```

```js
export const UNTERMINATED = 0;
export const TERMINATED = 1;

export function done(status) {
  return status === TERMINATED;
}
```
- **A variant with fields is an object of its `TAG`** and its fields: a
  tuple variant's are `_0`, `_1` ([ADR 0033](../../decisions/0033-enums-with-fields.md)).

Where a JS API has its own shape for a union, say so on the enum.

```rust
#[cfg_attr(rust_js, rust_js::tag = "status")]
pub enum Load {
    #[cfg_attr(rust_js, rust_js::name = "pending")]
    Pending,
    #[cfg_attr(rust_js, rust_js::name = "done")]
    Done { value: u32 },
}

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Input<'a> {
    Text(&'a str),
    Count(f64),
    #[cfg_attr(rust_js, rust_js::otherwise)]
    Other(&'a js::Unknown),
}

pub fn loads() -> [Load; 2] {
    [Load::Pending, Load::Done { value: 3 }]
}

pub fn size(input: Input) -> f64 {
    match input {
        Input::Text(text) => text.len() as f64,
        Input::Count(n) => n,
        Input::Other(_) => 0.0,
    }
}
```

```js
import { $byteLen } from "@rust-js/runtime";

export function loads() {
  return [{ status: "pending" }, { status: "done", value: 3 }];
}

export function size(input) {
  if (typeof input === "string") {
    return $byteLen(input);
  }
  if (typeof input === "number") {
    return input;
  }
  return 0;
}
```

- **`#[rust_js::tag = "status"]` makes a discriminated union**: each variant
  is an object of the tag, a fieldless one too, `{ status: "pending" }`.
  Its variants' fields are named; a tuple variant, or a field named as the
  tag, is an error. A variant's tag may be `true` or `false`,
  `#[rust_js::name = true]`
  ([ADR 0284](../../decisions/0284-discriminated-unions.md)).
- **`#[rust_js::untagged]` makes each value its payload**, as TypeScript's
  `string | number` is. `match` tests each payload's kind, `typeof`,
  `Array.isArray` or `instanceof`, so each variant holds one value, and no
  two of one kind. One without fields is its name's string, or `undefined`,
  `#[rust_js::undefined]`, or `null`, `#[rust_js::null]`, as `js::Kind`'s
  are, each tested by `===`
  ([ADR 0676](../../decisions/0676-kind-tells-undefined-and-null.md)).
  The last may be `#[rust_js::otherwise]`: whatever the
  others aren't ([ADR 0214](../../decisions/0214-untagged-enums.md)); where
  that's one kind `typeof` tells, as `js::Kind::Object`'s `"object"`, it's
  tested as that kind. A `match` of three cases or more of kinds `typeof`
  tells is a `switch (typeof value)`, as a `match` of constants is a
  `switch` ([ADR 0668](../../decisions/0668-a-match-of-what-a-value-is-is-a-switch-of-its-typeof.md)).

## Option and Result

```rust
pub struct Profile {
    pub name: String,
    pub nickname: Option<String>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub avatar: Option<String>,
    #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
    pub banner: Option<String>,
}

pub fn profile(name: &str) -> Profile {
    Profile {
        name: name.to_string(),
        nickname: None,
        avatar: None,
        banner: None,
    }
}

pub fn hide_banner(profile: &mut Profile) {
    profile.banner = None;
}

pub fn shown(profile: &Profile) -> &str {
    profile.nickname.as_deref().unwrap_or(&profile.name)
}

pub fn first(items: &[u32]) -> Option<u32> {
    items.first().copied()
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn last(items: &[u32]) -> Option<u32> {
    items.last().copied()
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn last_of(a: &[u32], b: &[u32]) -> Option<u32> {
    if b.is_empty() {
        return last(a);
    }
    last(b)
}

pub fn found(flag: bool) -> Option<Option<u32>> {
    if flag { Some(None) } else { None }
}

pub fn ratio(a: u32, b: u32) -> Result<u32, String> {
    if b == 0 {
        return Err("divide by zero".to_string());
    }
    Ok(a / b)
}
```

```js
import { $div, $some } from "@rust-js/runtime";

export function profile(name) {
  return {
    name,
    avatar: null,
  };
}

export function hideBanner(profile) {
  profile.banner = null;
}

export function shown(profile) {
  return profile.nickname ?? profile.name;
}

export function first(items) {
  return items[0];
}

export function last(items) {
  return items.at(-1) ?? null;
}

export function lastOf(a, b) {
  if (b.length === 0) {
    return last(a);
  }
  return last(b);
}

export function found(flag) {
  if (flag) {
    return $some(undefined);
  }
}

export function ratio(a, b) {
  if (b === 0) {
    return { TAG: "Err", _0: "divide by zero" };
  }
  return { TAG: "Ok", _0: $div(a, b) >>> 0 };
}
```

- **`Some(x)` is `x`, and `None` is `undefined`.** A `None` field is left
  out of the object. Reading one, rust-js tests `== null`, so a JS caller
  may pass `null` too ([ADR 0030](../../decisions/0030-option.md)).
- **`#[rust_js::nullable]` makes a `None` `null`**: on a field, on a
  function for its result, or on a function for a parameter it names,
  `#[rust_js::nullable(range)]`, a binding's or the crate's own
  ([ADR 0275](../../decisions/0275-nullable-writes-null.md),
  [ADR 0578](../../decisions/0578-a-nullable-result-where-its-written.md)).
  The crate's function's parameter is `null` where a call gives it `None`
  and where its body sets it so, `children = None`'s `children = null`,
  and what a call gives it otherwise is as it is, its body reading either
  as `None`; its `.d.ts` is `T | null | undefined`
  ([ADR 0861](../../decisions/0861-a-functions-own-nullable-parameter-is-set-null.md)).
  A `Some(..)` given to one is its value, `Object.create(Object.prototype)`.
  A result is `?? null` of an `Option` that may be `undefined`, and a
  nullable function's call as it is, `null` for `None` already, `return
  last(b);` ([ADR 0638](../../decisions/0638-a-nullable-functions-call-is-null-already.md)).
  Its `.d.ts` says so, `string | null`, never `undefined`, where another
  `Option`'s result is `string | null | undefined`
  ([ADR 0574](../../decisions/0574-what-reacts-scheduler-heap-asks.md), amended).
  A variable only nullable fields and parameters read is `null` where
  it's made, `let owner = None;`'s `const owner = null;`, and given as it
  is ([ADR 0781](../../decisions/0781-a-none-only-nullables-read-is-null-where-its-made.md)).
  A field's is of an `Option`, or of a `Cell` of one, whose `None` is
  `null` where it's made, where it's assigned, `profile.banner = null`,
  and where its cell is `set`, `replace`d or `take`n, `context.renderer =
  null`
  ([ADR 0772](../../decisions/0772-a-nullable-cell-field-writes-null.md),
  [ADR 0810](../../decisions/0810-an-optional-nullable-field-is-absent-until-set.md)),
  through a `let r = &context.renderer;` or a helper given `&Cell` too: a
  handle on it stores `value ?? null`
  ([ADR 0791](../../decisions/0791-a-cells-storage-says-what-it-stores.md)).
  What's never `undefined` is stored as it is, an object made there or a
  `Some`'s value, `context.Consumer = { $$typeof: REACT_CONSUMER_TYPE,
  _context: context }`, and a `map`'s `None` is `null` where it's made,
  `createTask ? createTask("lazy()") : null`; so is a nullable
  function's result, a binding's too, kept in a variable or a `Drop`
  guard's field, `ReactSharedInternals.T = prevTransition;`, but not a
  trait's method's, whose impl may give `undefined`
  ([ADR 0841](../../decisions/0841-a-nullable-place-stores-what-is-never-undefined-as-it-is.md));
  on anything else it's an error
  ([ADR 0586](../../decisions/0586-a-nullable-field-is-an-option.md)):

  ```
  rust-js: `#[rust_js::nullable]` is of a field that's an `Option`, or a `Cell` of one, whose `None` is `null`; this one isn't
  ```

- **`#[rust_js::optional]` on a nullable field makes it absent until
  it's set**, Flow's and TypeScript's `banner?: string | null`: made with
  `None`, it's left out, `{ name, avatar: null }`, as a plain `Option`
  field is; made with what may be `None`, a variable or a call, and set to
  `None`, it's `null`, as a nullable field is, `{ banner: banner ?? null
  }`: absent, `null` or a value, never `undefined` and there. The two
  marks are the two attributes: `nullable` the `| null`, `optional` the
  `?`. Read, absent, `null` or a value, it's its `Option`. Its `.d.ts` is a
  nullable field's. On a field that isn't nullable it's an error, as a
  plain `Option` field's `None` is left out already
  ([ADR 0810](../../decisions/0810-an-optional-nullable-field-is-absent-until-set.md)):

  ```
  rust-js: `#[rust_js::optional]` is of a `#[rust_js::nullable]` field, whose `None` is then left out where its struct is made and `null` where it's set; an `Option` field's `None` is left out already
  ```

- **A `Some` of what looks like `None` is boxed**: `Some(None)`, `Some(())`
  or a generic `T` that's either, `{ $someNone: 0 }`, so it isn't
  `undefined` ([ADR 0051](../../decisions/0051-generic-options.md)).
- **A `Result` is an object of its `TAG`**, `{ TAG: "Ok", _0: v }`, and
  `Ok(())` is `{ TAG: "Ok" }` ([ADR 0035](../../decisions/0035-results-and-throwing-js.md)).
  A binding returns one to say JS may throw: see
  [Errors, panics and exceptions](exceptions.md).

## Box, Rc and Arc

A `Box<T>`, an `Rc<T>` and an `Arc<T>` are the value itself: JS objects are
shared already.

```rust
use std::cell::RefCell;
use std::rc::Rc;

pub struct Node {
    pub name: String,
    pub children: RefCell<Vec<Rc<Node>>>,
}

pub fn node(name: &str) -> Rc<Node> {
    Rc::new(Node {
        name: name.to_string(),
        children: RefCell::new(Vec::new()),
    })
}

pub fn adopt(parent: &Node, child: &Rc<Node>) {
    parent.children.borrow_mut().push(Rc::clone(child));
}

pub fn same(a: &Rc<Node>, b: &Rc<Node>) -> bool {
    Rc::ptr_eq(a, b)
}
```

```js
export function node(name) {
  return {
    name,
    children: [],
  };
}

export function adopt(parent, child) {
  parent.children.push(child);
}

export function same(a, b) {
  return a === b;
}
```

A clone of a parameter's `Rc` moved into a closure is the same object, so
the closure reads it by the parameter's name
([ADR 0846](../../decisions/0846-a-parameters-clone-is-read-by-its-name.md)):

```rust
use std::cell::Cell;
use std::rc::Rc;

pub struct Payload {
    pub status: Cell<i32>,
}

pub fn handlers(payload: &Rc<Payload>) -> (Box<dyn Fn()>, Box<dyn Fn()>) {
    let resolved = Rc::clone(payload);
    let rejected = Rc::clone(payload);
    (
        Box::new(move || resolved.status.set(1)),
        Box::new(move || rejected.status.set(2)),
    )
}
```

```js
export function handlers(payload) {
  return [
    () => {
      payload.status = 1;
    },
    () => {
      payload.status = 2;
    },
  ];
}
```

A `Weak` that nothing upgrades or counts is what its `Rc` is, the object
itself, and `Weak::new()` is `null`: the `Weak`s a self-referring object
holds, as React's context is its own `Provider` and its `Consumer`'s
`_context`, are that object, set after it's made
([ADR 0843](../../decisions/0843-a-weak-nothing-upgrades-is-the-value-itself.md)).

```rust
use std::cell::Cell;
use std::rc::{Rc, Weak};

pub struct Context {
    pub value: Cell<f64>,
    pub provider: Cell<Weak<Context>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub consumer: Cell<Option<Rc<Consumer>>>,
}

pub struct Consumer {
    pub context: Weak<Context>,
}

pub fn create_context(value: f64) -> Rc<Context> {
    let context = Rc::new(Context {
        value: Cell::new(value),
        // These are circular
        provider: Cell::new(Weak::new()),
        consumer: Cell::new(None),
    });
    context.provider.set(Rc::downgrade(&context));
    context.consumer.set(Some(Rc::new(Consumer {
        context: Rc::downgrade(&context),
    })));
    context
}
```

```js
export function createContext(value) {
  const context = {
    value,
    // These are circular
    provider: null,
    consumer: null,
  };
  context.provider = context;
  context.consumer = {
    context,
  };
  return context;
}
```

An `Rc` whose counts the crate reads, `strong_count`, `weak_count` or a
`Weak`'s `upgrade`, is counted as Rust counts it: `{ value, strong, weak }`
([ADR 0320](../../decisions/0320-counted-rcs.md)).

```rust
use std::rc::Rc;

pub struct Doc {
    pub title: String,
}

pub fn open(title: &str) -> Rc<Doc> {
    Rc::new(Doc {
        title: title.to_string(),
    })
}

pub fn owners(doc: &Rc<Doc>) -> usize {
    Rc::strong_count(doc)
}

pub fn share(doc: &Rc<Doc>) -> Rc<Doc> {
    Rc::clone(doc)
}
```

```js
import { $rcClone } from "@rust-js/runtime";

export function open(title) {
  return {
    value: {
      title,
    },
    strong: 1,
    weak: 0,
  };
}

export function owners(doc) {
  return doc.strong;
}

export function share(doc) {
  return $rcClone(doc);
}
```

- **`Rc::ptr_eq` of a struct with fields counts nothing**, but a
  `#[repr(transparent)]` one's, which is its field: it's `a === b`
  ([ADR 0582](../../decisions/0582-ptr-eq-of-an-object-is-its-identity.md)).
- **Each type's `Rc`s are counted apart**
  ([ADR 0852](../../decisions/0852-each-types-rcs-are-counted-apart.md)):
  an `Rc` is counted where the crate reads the count of its type's, or of
  a type that may be the same, or its last owner runs a destructor.
  Generic code's `Rc<Slot<T>>` stands for every `Rc<Slot<_>>`, and an
  `Rc<T>` for every `Rc`. A generic `Rc` of a type parameter's value that
  may have a destructor is counted where a body owns one, not behind a
  reference or a `Weak`: React's `use_context<T>(context:
  &ReactContext<T>)` only reads its context and counts nothing, so
  `createContext` stays the value itself beside a `share<T>` whose
  `Rc<Slot<T>>`s are counted.
- **A rust-js library's contract lists the types whose `Rc`s it counts**,
  so one handle crosses crates
  ([ADR 0523](../../decisions/0523-libraries-share-one-counted-rc.md)):
  a crate a library hands an `Rc` counts it where the library does. It's
  refused where it would count one the library hands as the value itself
  ([ADR 0844](../../decisions/0844-a-library-that-reads-no-count-counts-no-rc.md)),
  `Rc::strong_count` of a library's `Rc<String>` that the library counts
  none of: read the count in the library, which then counts it. A library
  hands what the items of its the crate uses reach, an `Rc` in a
  function's parameters or result or a type's fields, not one of a type
  parameter the crate gives it, nor of a `bool` static's
  ([ADR 0626](../../decisions/0626-what-a-library-hands-a-crate.md)).
- **An `Rc` is `js::Defined` as what it points at is**, as a `Box` is, so
  a `WeakMap` or a `Map` holds one, `WeakMap<JsObject, Rc<CacheNode>>`,
  the value itself, which its `get` gives back as a clone shares it. A
  counted one given to a binding or taken from one is refused: JS would
  hold its box, which counts no clone JS gives back
  ([ADR 0850](../../decisions/0850-an-rc-is-defined-as-what-it-points-at.md)).

## Cell and RefCell

```rust
use std::cell::{Cell, RefCell};

pub struct Counter {
    pub hits: Cell<u32>,
    pub log: RefCell<Vec<String>>,
}

pub fn hit(counter: &Counter, line: &str) {
    counter.hits.set(counter.hits.get() + 1);
    counter.log.borrow_mut().push(line.to_string());
}

pub fn doubled() -> u32 {
    let total = Cell::new(1);
    total.set(total.get() * 2);
    total.get()
}

pub fn shared() -> RefCell<Vec<u32>> {
    let list = RefCell::new(Vec::new());
    let first = list.borrow();
    let empty = first.is_empty();
    drop(first);
    if empty {
        list.borrow_mut().push(1);
    }
    list
}
```

```js
export function hit(counter, line) {
  counter.hits = (counter.hits + 1) >>> 0;
  counter.log.push(line);
}

export function doubled() {
  let total = 1;
  total = (total * 2) >>> 0;
  return total;
}

export function shared() {
  const list = [];
  const first = list;
  const empty = first.length === 0;
  if (empty) {
    list.push(1);
  }
  return list;
}
```

- **A `Cell` in a field is the property it holds**: `counter.hits`, read and
  set in place ([ADR 0288](../../decisions/0288-cell-fields.md)).
- **A `RefCell` in a field is the value it holds** where every borrow of it
  in the crate lasts a moment, as `borrow_mut().push(..)`, `replace` and
  `take` do, and where none is borrowed at all: what decides is the field
  and its type, not whether some body borrows one
  ([ADR 0362](../../decisions/0362-refcell-fields-never-counted.md),
  [ADR 0585](../../decisions/0585-a-refcells-replace-holds-it-for-a-moment.md),
  [ADR 0863](../../decisions/0863-a-refcell-of-an-object-is-the-object.md)).
  A `#[rust_js::nullable]` one's `None` is `null` where it's made,
  `child: null`.
  A `#[repr(transparent)]` struct's field isn't a property: the struct is
  the cell.
- **A `RefCell` of an array, a `Map`, a `Set` or a struct's object is that
  object**, `list` above, its whole writes, `replace`, `take`, `*cell.get_mut()
  = v`, made in place; of a number, a string or an `Option` it's `{ value }`.
  Of a `#[repr(transparent)]` struct it's its field's: a `Vec`'s newtype's
  is the array.
  Generic code's `RefCell<T>` of any `T`, and one made a `RefCell<dyn T>`,
  is `{ value }` too, for every `T`
  ([ADR 0863](../../decisions/0863-a-refcell-of-an-object-is-the-object.md)).
- **A `RefCell` held across code counts its borrows**, as std's does: a
  second `borrow_mut()` while one is held panics
  ([ADR 0328](../../decisions/0328-refcell-borrow-counts.md)). One no borrow
  of which is ever held while code that may borrow it runs is checked
  nowhere, as none can fail
  ([ADR 0863](../../decisions/0863-a-refcell-of-an-object-is-the-object.md)).
- **A library's manifest says what it does with its `RefCell`s**, and a
  crate using it that would read one otherwise is refused: one it shares as
  the object, which the crate's generic code would box, or a borrow of one
  it never checks held while the library's code may run
  ([ADR 0863](../../decisions/0863-a-refcell-of-an-object-is-the-object.md)).
- **A local `Cell` whose cell no one else sees is a plain variable.**
- **A `Mutex` and an `RwLock` are cells too**, `{ value }`.

A `thread_local!` of a cell is a module variable: see
[Access to the JavaScript global scope](global-scope.md#module-variables).

## OnceCell and OnceLock

```rust
use std::cell::OnceCell;
use std::rc::Rc;

pub struct Info {
    pub name: String,
}

pub struct Payload {
    pub status: u32,
    pub info: OnceCell<Rc<Info>>,
}

pub fn payload(dev: bool) -> Rc<Payload> {
    let payload = Rc::new(Payload {
        status: 0,
        info: OnceCell::new(),
    });
    if dev {
        let info = Rc::new(Info {
            name: "lazy".to_string(),
        });
        let _ = payload.info.set(info);
    }
    payload
}

pub fn name(payload: &Payload) -> Option<String> {
    payload.info.get().map(|info| info.name.clone())
}

pub fn rename(payload: &Payload, name: &str) -> bool {
    let info = Rc::new(Info {
        name: name.to_string(),
    });
    payload.info.set(info).is_ok()
}
```

```js
export function payload(dev) {
  const payload = {
    status: 0,
  };
  if (dev) {
    const info = {
      name: "lazy",
    };
    payload.info = info;
  }
  return payload;
}

export function name(payload) {
  return payload.info?.name;
}

export function rename(payload, name) {
  const info = {
    name,
  };
  const unset = payload.info === undefined;
  if (unset) {
    payload.info = info;
  }
  return (unset ? { TAG: "Ok" } : { TAG: "Err", _0: info }).TAG === "Ok";
}
```

- **A `OnceCell` in a field is the property its `Option` is**, left out
  where it's made `OnceCell::new()` and `undefined` until it's set:
  `payload.info`, read where `get()` reads it, declared `info?: Info |
  undefined` ([ADR 0847](../../decisions/0847-a-oncecell-field-is-absent-until-set.md)).
- **A `set` of a cell the function made, that nothing could have set
  since, is an assignment**, `payload.info = info`; any other `set` tests
  it first, `if (payload.info === undefined)`, and where its `Result` is
  read, it's `Err` of the value where the cell was set already.
- **A `OnceCell` of its own is `{ value }`**, and one only its function
  reads and sets is its `let`. `get_or_init` and `get_mut` of a field
  have a handle on it.
- What's set to a cell of a `()` or an `Option`, which look like `None`,
  is boxed, so the cell set is never `undefined`
  ([ADR 0317](../../decisions/0317-once-cells.md)).

## References

```rust
pub struct Task {
    pub sort_index: u32,
    pub label: String,
}

pub fn bump(n: &mut u32) {
    *n += 1;
}

pub fn rename(task: &mut Task, label: &str) {
    task.label = label.to_string();
    bump(&mut task.sort_index);
}
```

```js
export function bump(n) {
  n.value = (n.value + 1) >>> 0;
}

export function rename(task, label) {
  task.label = label;
  const n = { value: task.sortIndex };
  bump(n);
  task.sortIndex = n.value;
}
```

- **A `&T` is the value.**
- **A `&mut` to an object is the object**, changed in place.
- **A `&mut` to what JS can't change in place**, a number, a string, an
  `Option` or a fieldless enum, is a box, `{ value }`, that the caller reads
  back. A JS caller of `bump` passes `{ value: 1 }`
  ([ADR 0099](../../decisions/0099-mut-references.md)).

## Functions and trait objects

```rust
pub trait Shape {
    fn area(&self) -> f64;
}

pub struct Square {
    pub side: f64,
}

impl Shape for Square {
    fn area(&self) -> f64 {
        self.side * self.side
    }
}

pub fn shapes() -> Vec<Box<dyn Shape>> {
    vec![Box::new(Square { side: 2.0 })]
}

pub fn adder(by: i32) -> impl Fn(i32) -> i32 {
    move |x| x + by
}

pub fn apply(f: fn(u32) -> u32, x: u32) -> u32 {
    f(x)
}

pub fn each(items: &[u32], f: &dyn Fn(u32)) {
    for &item in items {
        f(item);
    }
}
```

```js
export const squareShape = { area: squareShape_area };
function squareShape_area(square) {
  return square.side * square.side;
}

export function shapes() {
  return [{ value: { side: 2 }, impl: squareShape }];
}

export function adder(by) {
  return (x) => (x + by) | 0;
}

export function apply(f, x) {
  return f(x);
}

export function each(items, f) {
  for (const item of items) {
    f(item);
  }
}
```

- **A closure, an `fn` pointer, a `dyn Fn` and an `impl Fn` are a JS
  function**, an arrow where it's made. One JS gives is a `&'static dyn
  Fn` of its signature, and `std::ptr::eq` of two is `===`
  ([ADR 0653](../../decisions/0653-a-js-function-value-is-a-dyn-fn-of-its-signature.md)).
- **A `dyn Trait` is `{ value, impl }`**: the value, and its impl's
  dictionary of methods ([ADR 0049](../../decisions/0049-traits-and-generics.md)).

## JS values: JsObject and Unknown

A JS object you get from a binding is a struct that holds a
`PhantomData<JsObject>`. It's the object itself; you only have it by
reference ([ADR 0111](../../decisions/0111-js-types-as-structs.md)). See
[Write bindings for JavaScript APIs](bindings.md#declare-a-js-type).

A JS value of any shape, TypeScript's `unknown`, is a `js::Unknown`. It's
never `undefined` or `null`: an `Option<&Unknown>` is `None` of those.
`js::classify` tells what an `Option<&Unknown>` is, as a `js::Kind` to
`match` ([ADR 0225](../../decisions/0225-unknown-values.md)): a variant
for each kind `typeof` tells, `Undefined`, `String`, `Number`, `BigInt`,
`Bool`, `Symbol`, `Function` and `Object`, an object neither `null` nor
an array, a `&JsObject`, TypeScript's `object`, which reads as the
`Unknown` it is and keys a weak collection
([ADR 0677](../../decisions/0677-a-weak-key-is-what-kind-holds.md));
`Null`, which `=== null` does, and `Array`, which
`Array.isArray` does ([ADR 0676](../../decisions/0676-kind-tells-undefined-and-null.md));
of one that's there, `classify(Some(value))`. A `match` of them is a
`switch (typeof value)`, an array an `if` before it, `null` one in `case
"object"`, and `undefined` or `null` an `if (value == null)` before it,
and `Kind::Object(_) | Kind::Array(_)`, in a `match` or an `if let`, is an
object not `null`, `typeof value === "object" && value !== null`
([ADR 0668](../../decisions/0668-a-match-of-what-a-value-is-is-a-switch-of-its-typeof.md)).

```rust
use js::{Kind, Unknown};

pub fn describe(value: Option<&Unknown>) -> String {
    match js::classify(value) {
        Kind::Undefined | Kind::Null => "nothing".to_string(),
        Kind::String(text) => format!("text of {} bytes", text.len()),
        Kind::Number(n) => format!("the number {n}"),
        Kind::BigInt(n) => format!("the bigint {n}"),
        Kind::Bool(b) => format!("{b}"),
        Kind::Symbol(symbol) => symbol.to_string(),
        Kind::Function(_) => "a function".to_string(),
        Kind::Array(items) => format!("{} items", items.len()),
        Kind::Object(object) => match js::get(object, "name") {
            Some(name) => format!("an object named {}", js::string(Some(name))),
            None => "an object".to_string(),
        },
    }
}
```

```js
import { $byteLen, $displayF64 } from "@rust-js/runtime";

export function describe(value) {
  if (value == null) {
    return "nothing";
  }
  if (Array.isArray(value)) {
    return `${value.length} items`;
  }
  switch (typeof value) {
    case "string":
      return `text of ${$byteLen(value)} bytes`;
    case "number":
      return `the number ${$displayF64(value)}`;
    case "bigint":
      return `the bigint ${value}`;
    case "boolean":
      return String(value);
    case "symbol":
      return value.toString();
    case "function":
      return "a function";
    case "object": {
      const name = value.name;
      if (name != null) {
        return `an object named ${String(name)}`;
      }
      return "an object";
    }
  }
}
```

- `js::get(value, key)` is `value[key]`, `js::set(value, key, to)` is
  `value[key] = to`, `js::has(value, key)` is `key in value` and
  `js::delete(value, key)` `delete value[key]`, each of any value, as
  TypeScript's `any` is, an `Option<&Unknown>` too: of `None`, `undefined`,
  each throws JS's `TypeError`, a panic, as upstream's `'default' in
  moduleObject` does ([ADR 0776](../../decisions/0776-get-has-set-take-any-value.md)).
  Where a value may be missing and that's no error, Rust says so,
  `value.and_then(|v| js::get(v, key))`;
  `js::object::has_own_property.call(value, key)` is
  `Object.prototype.hasOwnProperty.call(value, key)`, a property of its own
  ([ADR 0615](../../decisions/0615-has-own-property-the-function.md)).
- `js::object::keys(value)` is `Object.keys(value)`, its own enumerable
  keys; `js::keys_in(value)` the keys `for…in` visits, its prototypes'
  too, each as the loop reaches it. A `for` loop of it is `for (const key
  in value)`, and of its `enumerate()` a counter each turn ends with,
  where no closure in it reads the count; anywhere else it's a JS iterator
  of them ([ADR 0719](../../decisions/0719-for-in-is-a-for-loop-of-keys-in.md)):

```rust
use js::Unknown;

pub fn removed(prev: &Unknown, next: &Unknown) -> Vec<String> {
    let mut gone = Vec::new();
    for (checked, key) in js::keys_in(prev).enumerate() {
        if checked > 100 {
            break;
        }
        if !js::has(next, &key) {
            gone.push(key);
        }
    }
    gone
}
```

```js
export function removed(prev, next) {
  const gone = [];
  let checked = 0;
  for (const key in prev) {
    if (checked > 100) {
      break;
    }
    if (!(key in next)) {
      gone.push(key);
    }
    checked++;
  }
  return gone;
}
```
- `js::new(value, arguments)` is `new value(..)`, its arguments a tuple,
  `()` of none, or a slice spread, as a rest parameter's
  ([ADR 0695](../../decisions/0695-new-of-a-function-value-is-jss-new.md)):

```rust
use js::Unknown;

pub fn made(class: &Unknown, name: &str) -> &'static Unknown {
    js::new(class, (name, 1))
}
```

```js
export function made(class$, name) {
  return new class$(name, 1);
}
```
- `js::delete(value, key)` is `delete value[key]`, whether it's gone
  ([ADR 0697](../../decisions/0697-delete-is-jss-delete.md)):

```rust
pub fn forget(props: &js::Unknown) {
    js::delete(props, "ref");
}
```

```js
export function forget(props) {
  delete props.ref;
}
```
- **A function is a value of any shape too**, a function item, a closure,
  a `fn` pointer or a `dyn Fn`: `js::unknown(&f)` is `f`, whose
  property `js::set` writes, here when its module loads, right after it
  ([ADR 0673](../../decisions/0673-a-function-is-a-value-of-any-shape.md)):

```rust
fn disabled_log() {}
js::on_load! {
    js::set(js::unknown(&disabled_log), "__reactDisabledLog", true);
}

pub fn is_marked(log: fn()) -> bool {
    js::get(js::unknown(&log), "__reactDisabledLog").is_some()
}
```

```js
function disabledLog() {}
disabledLog.__reactDisabledLog = true;

export function isMarked(log) {
  return log.__reactDisabledLog != null;
}
```
- **What every function has, `Function.prototype`'s, is `js::function`'s**,
  of a function of any signature, `js::AnyFunction`, as TypeScript types
  each: `function::name(f)` is `f.name`, a `String`, `function::length(f)`
  `f.length`, `function::to_string(f)` its source,
  `Function.prototype.toString.call(f)`, `function::call(f, this_arg,
  args)` `f.call(thisArg, ...args)`, and `function::apply(f, this_arg,
  arg_array)` `f.apply(thisArg, argArray)`, whose `this` is any value,
  `None` of `undefined` and `Some(&Kind::Null)`, the variant that is it,
  of `null`: `function::apply(r#fn, Some(&Kind::Null), Some(&args))` is
  `fn.apply(null, args)`
  ([ADRs 0740](../../decisions/0740-a-functions-members-are-read-of-any-function.md),
  [0848](../../decisions/0848-apply-is-functions-and-its-this-may-be-null.md)):

```rust
use js::{Kind, Unknown, function};

pub fn described(value: Option<&Unknown>) -> String {
    match js::classify(value) {
        Kind::Function(f) => {
            let function_name = function::name(f);
            if function_name.is_empty() {
                "() => {}".to_string()
            } else {
                function_name + "() {}"
            }
        }
        _ => String::new(),
    }
}

pub fn same_source(prev: &dyn Fn(&str), next: &dyn Fn(&str)) -> bool {
    function::length(prev) == function::length(next) && function::to_string(prev) == function::to_string(next)
}
```

```js
export function described(value) {
  if (typeof value === "function") {
    const functionName = value.name;
    return !functionName ? "() => {}" : functionName + "() {}";
  }
  return "";
}

export function sameSource(prev, next) {
  return (
    prev.length === next.length &&
    Function.prototype.toString.call(prev) === Function.prototype.toString.call(next)
  );
}
```
- **A constructor is a `fn` item**, a function declaration, which `new`
  constructs and which has a `prototype`, declared where it's written, in
  the block it's written in; a closure is an arrow
  function, which `new` throws a `TypeError` of
  ([ADR 0696](../../decisions/0696-a-constructor-is-a-fn-item-where-its-written.md)):

```rust
use js::Unknown;

pub fn sample(construct: bool) -> Option<&'static Unknown> {
    if construct {
        // What the props are set on.
        #[cfg_attr(rust_js, rust_js::name = "Fake")]
        fn fake() -> Option<&'static Unknown> {
            None
        }
        return js::get(js::new(js::unknown(&fake), ()), "props");
    }
    None
}
```

```js
export function sample(construct) {
  if (construct) {
    // What the props are set on.
    function Fake() {}
    return new Fake().props;
  }
}
```
- **`==` of two is JS's `===`**; of two `Option<&Unknown>`, std's
  `Option`'s: both `None`, whichever of `null` and `undefined` each is,
  or `Some`s `===` finds equal, `(a ?? null) === (b ?? null)`, and `a ===
  b` where one is a `Some`; `js::classify` tells a `null` from an
  `undefined`. A struct, a tuple or a slice holding one compares it so,
  never by what's in it
  ([ADR 0612](../../decisions/0612-unknown-equality-is-strict.md)):

```rust
pub fn same(a: Option<&js::Unknown>, b: Option<&js::Unknown>) -> bool {
    a == b
}
```

```js
export function same(a, b) {
  return (a ?? null) === (b ?? null);
}
```
- **`js::strict_equals(a, b)` is JS's `===`**, of any two values, `None`s
  too, where a `null` isn't an `undefined`
  ([ADR 0721](../../decisions/0721-strict-equals-is-jss-triple-equals.md)).
  A port writes `==` where the Rust means its values are equal, and
  `js::strict_equals` where the JS it ports asks `===` of what may be
  either `None`, as React's `Object.is` polyfill does:

```rust
pub fn is(a: Option<&js::Unknown>, b: Option<&js::Unknown>) -> bool {
    js::strict_equals(a, b)
}
```

```js
export function is(a, b) {
  return a === b;
}
```
- `js::Json` is a JSON value, and `js::Dict<T>` a plain object of `T`s by
  name, TypeScript's `Record<string, T>`.

## Symbols

A symbol of JS's registry, `Symbol.for(key)`, is a `js::SymbolFor`: a
value of its key, so a `const` holds one and a `match` names it, as
React's tags are. Two are equal where their keys are, as `===` finds
them; its key, `.0`, is `Symbol.keyFor(symbol)`. Any other symbol, one of
its own, `js::symbol::new("id")`, or a well-known one,
`js::symbol::iterator`, is a `&js::Symbol`
([ADR 0634](../../decisions/0634-a-registered-symbol-is-a-const.md)).

`js::SymbolFor::from_value(value)` reads a value of any shape as one:
`None` of any other value. Matched against the registry's symbols, its
patterns each a `Some` of them or `_`, it's the value itself, as no other
value is one of them, and a `match` of three cases or more is a `switch`
of it. An `==` or `!=` of it and a `Some` is that `matches!`, `===` and
`!==` ([ADR 0771](../../decisions/0771-an-equality-of-a-registered-symbol-is-its-match.md)):

```rust
use js::{SymbolFor, Unknown};

pub const REACT_FRAGMENT_TYPE: SymbolFor = SymbolFor("react.fragment");
pub const REACT_PROFILER_TYPE: SymbolFor = SymbolFor("react.profiler");
pub const REACT_MEMO_TYPE: SymbolFor = SymbolFor("react.memo");
pub const REACT_LAZY_TYPE: SymbolFor = SymbolFor("react.lazy");

pub fn name(ty: Option<&Unknown>) -> Option<&'static str> {
    match SymbolFor::from_value(ty) {
        Some(REACT_FRAGMENT_TYPE) => Some("Fragment"),
        Some(REACT_PROFILER_TYPE) => Some("Profiler"),
        _ => None,
    }
}

pub fn is_lazy(ty: &Unknown) -> bool {
    matches!(SymbolFor::from_value(js::get(ty, "$$typeof")), Some(REACT_LAZY_TYPE))
}

pub fn is_memo(ty: &Unknown) -> bool {
    SymbolFor::from_value(js::get(ty, "$$typeof")) == Some(REACT_MEMO_TYPE)
}

pub fn key(value: Option<&Unknown>) -> Option<&'static str> {
    SymbolFor::from_value(value).map(|symbol| symbol.0)
}
```

```js
import { $symbolFor } from "@rust-js/runtime";

export const REACT_FRAGMENT_TYPE = Symbol.for("react.fragment");
export const REACT_PROFILER_TYPE = Symbol.for("react.profiler");
export const REACT_MEMO_TYPE = Symbol.for("react.memo");
export const REACT_LAZY_TYPE = Symbol.for("react.lazy");

export function name(ty) {
  switch (ty) {
    case REACT_FRAGMENT_TYPE:
      return "Fragment";
    case REACT_PROFILER_TYPE:
      return "Profiler";
    default:
      return;
  }
}

export function isLazy(ty) {
  return ty.$$typeof === REACT_LAZY_TYPE;
}

export function isMemo(ty) {
  return ty.$$typeof === REACT_MEMO_TYPE;
}

export function key(value) {
  const symbol = $symbolFor(value);
  return symbol != null ? Symbol.keyFor(symbol) : undefined;
}
```

Where the program tells its `Some` from its `None`, a `None =>` arm or a
`Some(symbol)` it binds, it's checked, `$symbolFor(value)`: the value
where it's the registry's, `undefined` where not. `js::SymbolFor(key)`
is the one way to make one; `js::symbol::r#for` is gone, an error that
says so.

## Names

Every Rust name is camelCase in JS: a function, a field, a method, a
variant's field ([ADR 0573](../../decisions/0573-every-rust-name-is-camel-case-in-js.md)).

```rust
# pub struct Task {
#     pub sort_index: u32,
#     pub label: String,
# }
#
pub fn first_word(text: &str) -> &str {
    text.split(' ').next().unwrap_or("")
}

pub fn delete(tasks: &mut Vec<Task>, at: usize) {
    tasks.remove(at);
}
```

```js
import { $removeAt } from "@rust-js/runtime";

export function firstWord(text) {
  return text.split(" ")[0] ?? "";
}

export function delete$(tasks, at) {
  $removeAt(tasks, at);
}
```

- **A name JS reserves gets a `$`**: `delete$`
  ([ADR 0038](../../decisions/0038-js-names-and-destructuring.md)).
- **A parameter's leading `_`**, Rust's mark of one unused, isn't in its
  JS name, nor its declaration's: `_ref` is `ref`. Where what's left is a
  word JS keeps, `_new`, or another binding's of the function, `_x` beside
  an `x`, it stays; a closure beside it doesn't count, `|_instance| false`
  beside `|instance, _callback| ..` is `(instance) => false`
  ([ADR 0678](../../decisions/0678-a-parameters-underscore-isnt-its-js-name.md)).
- **A `const` keeps its name**, `MAX_SIZE`, as JS writes a constant.
- **A `thread_local!` is named as a module variable**, `CLICK_COUNT` as
  `clickCount` ([ADR 0579](../../decisions/0579-a-thread-local-is-named-as-a-variable.md)).
- **A struct, an enum and a variant keep their names**, `Point`, `"Red"`.

Where JS needs another name, give it with `#[rust_js::name]`: a JSON key
that's `snake_case`, a JS keyword, a string a variant must be.

```rust
pub struct Manifest {
    #[cfg_attr(rust_js, rust_js::name = "short_name")]
    pub short_name: String,
    #[cfg_attr(rust_js, rust_js::name = "class")]
    pub class_name: String,
}
```

A `#[rust_js::name]` that repeats the camelCase name is an error, since
there's one way to write it:

```
rust-js: `sort_index` is `sortIndex` in JS without `#[rust_js::name = "sortIndex"]`: remove it
```

What a binding calls in JS is its `#[rust_js::link_name]`, not its Rust name:
see [Write bindings for JavaScript APIs](bindings.md).

## What TypeScript sees

`rust-js build` writes a `.d.ts` beside each module, of these same types
([ADR 0196](../../decisions/0196-typescript-declarations.md)):

```rust
pub struct Profile {
    pub name: String,
    pub nickname: Option<String>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub avatar: Option<String>,
    pub visits: u64,
}

pub fn greet(name: &str, greeting: Option<&str>) -> String {
    format!("{}, {name}", greeting.unwrap_or("Hello"))
}
```

```ts
export interface Profile {
    name: string;
    nickname?: string | undefined;
    avatar?: string | null | undefined;
    visits: bigint;
}

export function greet(name: string, greeting?: string | null | undefined): string;
```

A JS type's declaration is its `#[rust_js::types]`; one without is `any`.
See [Export Rust APIs to JavaScript](export-to-javascript.md).
