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
| fieldless enum variant | its name, `"Red"` | [Enums](#enums) |
| enum variant with fields | `{ TAG: "Circle", radius }`, `{ TAG: "Square", _0 }` | [Enums](#enums) |
| `#[rust_js::tag = "status"]` enum | `{ status: "done", value }` | [Enums](#enums) |
| `#[rust_js::untagged]` enum | its payload itself | [Enums](#enums) |
| `Option<T>` | the value, or `undefined` | [Option and Result](#option-and-result) |
| `#[rust_js::nullable]` `Option<T>` | the value, or `null` | [Option and Result](#option-and-result) |
| `Result<T, E>` | `{ TAG: "Ok", _0 }`, `{ TAG: "Err", _0 }` | [Option and Result](#option-and-result) |
| `Box<T>`, `Rc<T>`, `Arc<T>` | the value itself | [Pointers](#box-rc-and-arc) |
| an `Rc` whose count is read | `{ value, strong, weak }` | [Pointers](#box-rc-and-arc) |
| `Cell<T>`, `RefCell<T>` | `{ value }`; in a field, the property itself | [Cells](#cell-and-refcell) |
| `&T` | the value | [References](#references) |
| `&mut` to an object | the object, changed in place | [References](#references) |
| `&mut` to a number, a string, an `Option` | a box `{ value }` | [References](#references) |
| closure, `fn` pointer, `dyn Fn` | function | [Functions](#functions-and-trait-objects) |
| `dyn Trait` | `{ value, impl }` | [Functions](#functions-and-trait-objects) |
| a JS type, `struct X(PhantomData<JsObject>)` | the JS object | [JS values](#js-values-jsobject-and-unknown) |
| `js::Unknown` | any JS value | [JS values](#js-values-jsobject-and-unknown) |

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
  runtime's `$KeyMap`.
- **A `BTreeMap` is a `Map` and a `BTreeSet` a `Set`,** iterated in key
  order.

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
    return 3.141592653589793 * shape.radius * shape.radius;
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
  two of one kind. The last may be `#[rust_js::otherwise]`: whatever the
  others aren't ([ADR 0214](../../decisions/0214-untagged-enums.md)).

## Option and Result

```rust
pub struct Profile {
    pub name: String,
    pub nickname: Option<String>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub avatar: Option<String>,
}

pub fn profile(name: &str) -> Profile {
    Profile {
        name: name.to_string(),
        nickname: None,
        avatar: None,
    }
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
  return { name, avatar: null };
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
  function for its result, or on a binding for a parameter it names,
  `#[rust_js::nullable(range)]`
  ([ADR 0275](../../decisions/0275-nullable-writes-null.md),
  [ADR 0578](../../decisions/0578-a-nullable-result-where-its-written.md)).
  It's of an `Option` only; on anything else it's an error
  ([ADR 0586](../../decisions/0586-a-nullable-field-is-an-option.md)):

  ```
  rust-js: `#[rust_js::nullable]` is of a field that's an `Option`, whose `None` is `null`; this one isn't
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
  return { name, children: [] };
}

export function adopt(parent, child) {
  parent.children.push(child);
}

export function same(a, b) {
  return a === b;
}
```

An `Rc` whose counts the crate reads, `strong_count`, `weak_count` or a
`Weak`, is counted as Rust counts it: `{ value, strong, weak }`
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
  return { value: { title }, strong: 1, weak: 0 };
}

export function owners(doc) {
  return doc.strong;
}

export function share(doc) {
  return $rcClone(doc);
}
```

- **`Rc::ptr_eq` of a struct with fields counts nothing**: it's `a === b`
  ([ADR 0582](../../decisions/0582-ptr-eq-of-an-object-is-its-identity.md)).
- **A crate that is a rust-js library, or uses one, counts every `Rc`**, so
  one handle crosses crates ([ADR 0523](../../decisions/0523-libraries-share-one-counted-rc.md)).

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
import { $borrow, $borrowMut, $unborrow } from "@rust-js/runtime";

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
  const list = { value: [] };
  const first = $borrow(list, true);
  const empty = first.value.length === 0;
  $unborrow(first);
  if (empty) {
    const borrowMut = $borrowMut(list, true);
    try {
      borrowMut.value.push(1);
    } finally {
      $unborrow(borrowMut);
    }
  }
  return list;
}
```

- **A `Cell` in a field is the property it holds**: `counter.hits`, read and
  set in place ([ADR 0288](../../decisions/0288-cell-fields.md)).
- **A `RefCell` in a field is the value it holds** where every borrow of it
  in the crate lasts a moment, as `borrow_mut().push(..)`, `replace` and
  `take` do ([ADR 0362](../../decisions/0362-refcell-fields-never-counted.md),
  [ADR 0585](../../decisions/0585-a-refcells-replace-holds-it-for-a-moment.md)).
- **A `RefCell` held across code is `{ value }` and counts its borrows**, as
  std's does: a second `borrow_mut()` while one is held panics
  ([ADR 0328](../../decisions/0328-refcell-borrow-counts.md)).
- **A local `Cell` whose cell no one else sees is a plain variable.**
- **A `Mutex` and an `RwLock` are cells too**, `{ value }`.

A `thread_local!` of a cell is a module variable: see
[Access to the JavaScript global scope](global-scope.md#module-variables).

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
  function**, an arrow where it's made.
- **A `dyn Trait` is `{ value, impl }`**: the value, and its impl's
  dictionary of methods ([ADR 0049](../../decisions/0049-traits-and-generics.md)).

## JS values: JsObject and Unknown

A JS object you get from a binding is a struct that holds a
`PhantomData<JsObject>`. It's the object itself; you only have it by
reference ([ADR 0111](../../decisions/0111-js-types-as-structs.md)). See
[Write bindings for JavaScript APIs](bindings.md#declare-a-js-type).

A JS value of any shape, TypeScript's `unknown`, is a `js::Unknown`. It's
never `undefined` or `null`: an `Option<&Unknown>` is `None` of those.
`js::classify` tells what it is, as a `js::Kind` to `match`
([ADR 0225](../../decisions/0225-unknown-values.md)).

```rust
use js::{Kind, Unknown};

pub fn describe(value: Option<&Unknown>) -> String {
    let Some(value) = value else {
        return "nothing".to_string();
    };
    match js::classify(value) {
        Kind::String(text) => format!("text of {} bytes", text.len()),
        Kind::Number(n) => format!("the number {n}"),
        Kind::BigInt(n) => format!("the bigint {n}"),
        Kind::Bool(b) => format!("{b}"),
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
  if (typeof value === "string") {
    return `text of ${$byteLen(value)} bytes`;
  }
  if (typeof value === "number") {
    return `the number ${$displayF64(value)}`;
  }
  if (typeof value === "bigint") {
    return `the bigint ${value}`;
  }
  if (typeof value === "boolean") {
    return String(value);
  }
  if (typeof value === "function") {
    return "a function";
  }
  if (Array.isArray(value)) {
    return `${value.length} items`;
  }
  const name = value.name;
  if (name != null) {
    return `an object named ${String(name)}`;
  }
  return "an object";
}
```

- `js::get(value, key)` is `value[key]`, `js::set(value, key, to)` is
  `value[key] = to`, and `js::has(value, key)` is `key in value`;
  `js::object::has_own_property.call(value, key)` is
  `Object.prototype.hasOwnProperty.call(value, key)`, a property of its own
  ([ADR 0615](../../decisions/0615-has-own-property-the-function.md)).
- **`==` of two is JS's `===`**, of two `Option<&Unknown>` too, where a
  `null` isn't an `undefined`, unlike std's `Option`
  ([ADR 0612](../../decisions/0612-unknown-equality-is-strict.md)):

```rust
pub fn same(a: Option<&js::Unknown>, b: Option<&js::Unknown>) -> bool {
    a == b
}
```

```js
export function same(a, b) {
  return a === b;
}
```
- `js::Json` is a JSON value, and `js::Dict<T>` a plain object of `T`s by
  name, TypeScript's `Record<string, T>`.

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
