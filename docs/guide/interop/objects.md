# Structs, enums and traits as JavaScript sees them

A struct is a plain JS object, and an enum is a string or a tagged object.
There are no classes and no prototypes: a type's methods are functions in
an object named after it, and a trait's are a dictionary of functions.
This page shows each, with the JS rust-js writes for it.

Every example was compiled by rust-js as it is today. The JS shown is what
it writes, without its header comment and source-map line.

## Contents

- [Structs and enums](#structs-and-enums)
- [Methods](#methods)
- [Traits](#traits)
- [Generic code takes a dictionary](#generic-code-takes-a-dictionary)
- [`dyn Trait` is a value and its dictionary](#dyn-trait-is-a-value-and-its-dictionary)
- [Derives: `Debug`, `Clone`, `PartialEq`, `Default`](#derives-debug-clone-partialeq-default)
- [`Display` and operators](#display-and-operators)
- [Drop](#drop)
- [Iterators](#iterators)

## Structs and enums

| Rust | JS |
|---|---|
| `struct Point { x: f64, y: f64 }` | `{ x, y }`, its fields camelCase: `step_size` is `stepSize` |
| `struct Meters(f64)`, a tuple | an array, `[5]` |
| a unit struct, `()` | `undefined` |
| `enum Color { Red, Green }` | its variant's name, `"Red"` |
| `Status::Loading { percent: 40 }` | `{ TAG: "Loading", percent: 40 }` |
| `Status::Failed(message)` | `{ TAG: "Failed", _0: message }` |
| `Box<T>`, `Rc<T>` | the value itself |

Every shape, `Option` and `Result` among them, is in
[JavaScript types](types.md).

## Methods

```rust
pub struct Rect {
    pub width: f64,
    pub height: f64,
}

pub struct Meters(pub f64);

impl Rect {
    pub fn square(side: f64) -> Rect {
        Rect {
            width: side,
            height: side,
        }
    }

    pub fn area(&self) -> f64 {
        self.width * self.height
    }

    pub fn scale(&mut self, by: f64) {
        self.width *= by;
        self.height *= by;
    }
}

pub fn scaled_area(side: f64) -> Meters {
    let mut rect = Rect::square(side);
    rect.scale(2.0);
    Meters(rect.area())
}
```

```js
export const Rect = {
  square(side) {
    return {
      width: side,
      height: side,
    };
  },

  area(rect) {
    return rect.width * rect.height;
  },

  scale(rect, by) {
    rect.width *= by;
    rect.height *= by;
  },
};

export function scaledArea(side) {
  const rect = Rect.square(side);
  Rect.scale(rect, 2);
  return [Rect.area(rect)];
}
```

- **An `impl` block is an object named after its type**, in the module
  the `impl` is in. `Rect::square` is `Rect.square` ([ADR 0047](../../decisions/0047-methods.md)).
- **A method call is its full form**: `rect.scale(2.0)` is Rust's
  `Rect::scale(&mut rect, 2.0)`, so it's `Rect.scale(rect, 2)`.
- **`self` is named after its type**, `rect`. By value, `&self` or
  `&mut self`, it's the object itself; `&mut self` changes it in place.
- **The object is exported** where one of its methods is `pub`.

## Traits

```rust
pub trait Shape {
    fn area(&self) -> f64;

    fn describe(&self) -> String {
        format!("a shape of area {}", self.area())
    }
}

pub struct Circle {
    pub radius: f64,
}

pub struct Rect {
    pub width: f64,
    pub height: f64,
}

impl Shape for Circle {
    fn area(&self) -> f64 {
        std::f64::consts::PI * self.radius * self.radius
    }
}

impl Shape for Rect {
    fn area(&self) -> f64 {
        self.width * self.height
    }

    fn describe(&self) -> String {
        format!("a {}x{} rectangle", self.width, self.height)
    }
}

pub fn direct() -> String {
    Rect {
        width: 1.0,
        height: 1.0,
    }
    .describe()
}
```

```js
import { $displayF64 } from "@rust-js/runtime";

export const circleShape = {
  area: circleShape_area,
  describe: (self) => `a shape of area ${$displayF64(circleShape_area(self))}`,
};
function circleShape_area(circle) {
  return Math.PI * circle.radius * circle.radius;
}

export const rectShape = { area: rectShape_area, describe: rectShape_describe };
function rectShape_area(rect) {
  return rect.width * rect.height;
}

function rectShape_describe(rect) {
  return `a ${$displayF64(rect.width)}x${$displayF64(rect.height)} rectangle`;
}

export function direct() {
  return rectShape_describe({ width: 1, height: 1 });
}
```

- **A call on a known type is a direct call**: `rect.describe()` of a
  `Rect` is `rectShape_describe(rect)`. Nothing is looked up at run time.
- **Each impl has a dictionary**, an object of its methods, its module's
  `const`: `circleShape`, its type's name then its trait's
  ([ADR 0049](../../decisions/0049-traits-and-generics.md),
  [ADR 0592](../../decisions/0592-a-dictionary-is-its-modules-const.md)).
  An impl generic code gives its own dictionaries, `impl<T: Display>
  Display for Wrapper<T>`, is a function of them, `wrapperDisplay(TDisplay)`.
  It's exported where the trait is `pub`.
- **A default method is copied into each impl's dictionary** that doesn't
  write its own, so a call inside it reaches the impl's own methods.

## Generic code takes a dictionary

A function of a `T: Trait` takes the impl's dictionary after its
arguments, named after the parameter and the trait:

```rust
# mod shape {
#     pub trait Shape {
#         fn area(&self) -> f64;
#
#         fn describe(&self) -> String {
#             format!("a shape of area {}", self.area())
#         }
#     }
#
#     pub struct Circle {
#         pub radius: f64,
#     }
#
#     pub struct Rect {
#         pub width: f64,
#         pub height: f64,
#     }
#
#     impl Shape for Circle {
#         fn area(&self) -> f64 {
#             std::f64::consts::PI * self.radius * self.radius
#         }
#     }
#
#     impl Shape for Rect {
#         fn area(&self) -> f64 {
#             self.width * self.height
#         }
#
#         fn describe(&self) -> String {
#             format!("a {}x{} rectangle", self.width, self.height)
#         }
#     }
# }
# use shape::{Circle, Shape};
#
pub fn total_area<S: Shape>(shapes: &[S]) -> f64 {
    shapes.iter().map(|s| s.area()).sum()
}

pub fn circles() -> f64 {
    total_area(&[Circle { radius: 1.0 }, Circle { radius: 2.0 }])
}
```

```js
import { circleShape } from "./shape.js";

export function totalArea(shapes, SShape) {
  return shapes.map((s) => SShape.area(s)).reduce((a, b) => a + b, -0);
}

export function circles() {
  return totalArea([{ radius: 1 }, { radius: 2 }], circleShape);
}
```

A JS caller gives it the same way: `totalArea([{ radius: 1 }], circleShape)`.
A type parameter with no bound takes none.

## `dyn Trait` is a value and its dictionary

A `Box<dyn Shape>`, a `&dyn Shape`, is `{ value, impl }`: the value, and
its impl's dictionary ([ADR 0049](../../decisions/0049-traits-and-generics.md)).

```rust
# mod shape {
#     pub trait Shape {
#         fn area(&self) -> f64;
#
#         fn describe(&self) -> String {
#             format!("a shape of area {}", self.area())
#         }
#     }
#
#     pub struct Circle {
#         pub radius: f64,
#     }
#
#     pub struct Rect {
#         pub width: f64,
#         pub height: f64,
#     }
#
#     impl Shape for Circle {
#         fn area(&self) -> f64 {
#             std::f64::consts::PI * self.radius * self.radius
#         }
#     }
#
#     impl Shape for Rect {
#         fn area(&self) -> f64 {
#             self.width * self.height
#         }
#
#         fn describe(&self) -> String {
#             format!("a {}x{} rectangle", self.width, self.height)
#         }
#     }
# }
# use shape::{Circle, Rect, Shape};
#
pub fn largest(shapes: &[Box<dyn Shape>]) -> String {
    let mut best = &shapes[0];
    for shape in shapes {
        if shape.area() > best.area() {
            best = shape;
        }
    }
    best.describe()
}

pub fn mixed() -> String {
    let shapes: Vec<Box<dyn Shape>> = vec![
        Box::new(Circle { radius: 1.0 }),
        Box::new(Rect {
            width: 2.0,
            height: 3.0,
        }),
    ];
    largest(&shapes)
}
```

```js
import { circleShape, rectShape } from "./shape.js";
import { $index } from "@rust-js/runtime";

export function largest(shapes) {
  let best = $index(shapes, 0);
  for (const shape of shapes) {
    if (shape.impl.area(shape.value) > best.impl.area(best.value)) {
      best = shape;
    }
  }
  return best.impl.describe(best.value);
}

export function mixed() {
  const shapes = [
    { value: { radius: 1 }, impl: circleShape },
    {
      value: {
        width: 2,
        height: 3,
      },
      impl: rectShape,
    },
  ];
  return largest(shapes);
}
```

A closure, `Box<dyn Fn(u32) -> u32>`, is a JS function, not a pair. A
field that's a closure of statements is a method, `then(resolve,
reject) { .. }`, as JS writes a thenable; one of a value alone stays an
arrow, `double: (x) => x * 2`. A method and an arrow differ only in
their `this`, which a Rust closure never reads; one that reads a class's
`this` stays an arrow
([ADR 0882](../../decisions/0882-a-field-of-statements-is-a-method.md)):

```rust
pub struct Thenable {
    pub then: Box<dyn Fn(u32) -> u32>,
    pub double: Box<dyn Fn(u32) -> u32>,
}

pub fn thenable(k: u32) -> Thenable {
    Thenable {
        then: Box::new(move |n| {
            let sum = n + k;
            sum * 2
        }),
        double: Box::new(|n| n * 2),
    }
}
```

```js
export function thenable(k) {
  return {
    then(n) {
      const sum = (n + k) >>> 0;
      return (sum * 2) >>> 0;
    },
    double: (n) => (n * 2) >>> 0,
  };
}
```

## Derives: `Debug`, `Clone`, `PartialEq`, `Default`

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct User {
    pub name: String,
    pub tags: Vec<String>,
}

pub fn renamed(user: &User) -> User {
    let mut copy = user.clone();
    copy.name.push('!');
    copy.tags.push("copy".to_string());
    copy
}

pub fn equal(a: &User, b: &User) -> bool {
    a == b
}
```

```js
import { $eq } from "@rust-js/runtime";

export function renamed(user) {
  const copy = { ...user, tags: user.tags.slice() };
  copy.name += "!";
  copy.tags.push("copy");
  return copy;
}

export function equal(a, b) {
  return $eq(a, b);
}
```

- **`clone()` copies what needs copying**: the object, and inside it the
  arrays and objects that can change. A string can't, so it's shared.
- **A `Copy` value is copied only if its type is changed in place
  somewhere**, `{ ...p }`. Otherwise sharing it can't be told from
  copying it.
- **A derived `==` compares by value**, `$eq(a, b)`, field by field. One
  of your own `PartialEq` is its `eq`: `a != b` is `!celsiusPartialEq_eq(a, b)`.
- **A derived `Debug` is a function that makes Rust's text**,
  `Point { x: 1.0, y: 2.0 }`, called where `{:?}` is.
- **A derived `Default` is the object it makes**: `Point::default()` is
  `{ x: 0, y: 0 }`.
- **A derived `PartialOrd` or `Ord` compares field by field**: `sort()`
  of them is `versions.sort((a, b) => $cmp(a.major, b.major) || $cmp(a.minor, b.minor))`.

## `Display` and operators

```rust
use std::fmt;
use std::ops::Add;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Add for Point {
    type Output = Point;

    fn add(self, other: Point) -> Point {
        Point {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

pub fn moved(p: Point) -> Point {
    let mut q = p;
    q.x += 1.0;
    q + Point::default()
}

pub fn show(p: Point) -> String {
    format!("{p} is {p:?}")
}
```

```js
import { $debugF64, $displayF64 } from "@rust-js/runtime";

function pointDebug_fmt(point) {
  return `Point { x: ${$debugF64(point.x)}, y: ${$debugF64(point.y)} }`;
}

function pointAdd_add(point, other) {
  return {
    x: point.x + other.x,
    y: point.y + other.y,
  };
}

export const pointDisplay = { fmt: pointDisplay_fmt };
function pointDisplay_fmt(point) {
  return `(${$displayF64(point.x)}, ${$displayF64(point.y)})`;
}

export function moved(p) {
  const q = { ...p };
  q.x++;
  return pointAdd_add({ ...q }, { x: 0, y: 0 });
}

export function show(p) {
  return `${pointDisplay_fmt(p)} is ${pointDebug_fmt(p)}`;
}
```

- **`Display` is a function that gives the text**, and `{p}` is its
  call. A struct has no `toString()`: JS's `String(p)` is
  `"[object Object]"`. JS that wants Rust's text calls
  `pointDisplay.fmt(p)`. A [class's](export-to-javascript.md#js-classes)
  `Display` is its `toString()`.
- **An operator of your own is its method's call**: `q + other` is
  `pointAdd_add(q, other)`. An operator of numbers stays JS's, `+`.

## Drop

A `Drop` runs where Rust runs it: at the end of its scope, in reverse
order, and while a panic unwinds ([ADR 0098](../../decisions/0098-destructors.md)).

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

- **Code with no `Drop` impl has no extra JS.**
- **A value moved away isn't dropped where it was.** One moved into a
  function is dropped there, at its end:

```rust
pub struct Connection {
    pub id: u32,
}

impl Drop for Connection {
    fn drop(&mut self) {
        println!("closed {}", self.id);
    }
}

pub fn open(id: u32) -> Connection {
    Connection { id }
}

pub fn close(connection: Connection) {
    println!("closing {}", connection.id);
}
```

```js
export function open(id) {
  return { id };
}

export function close(connection) {
  console.log(`closing ${connection.id}`);
  console.log(`closed ${connection.id}`);
}
```

- **A `Drop` is its body where it's dropped**, at each drop, as a
  `finally` written by hand is
  ([ADR 0589](../../decisions/0589-a-drop-dropped-once-is-its-body-there.md)).
  A guard made only to be dropped, read only by its fields, is its fields:
  `let _restore = Restore { previous: DEPTH.get() };` is `const previous =
  depth;`, restored in the `finally`, a `let` where a field is written. A
  one-field tuple's is its field by the guard's name, `restore`. A field
  whose name another guard's took, `_outer.name` above, stays the guard's.

- **A value JS holds is JS's.** `open(1)` gives JS a `Connection` that
  nothing drops until JS gives it to `close`.

How a panic unwinds through them is
[Errors, panics and exceptions](exceptions.md#unwinding-runs-drop).

## Iterators

An `Iterator` of your own is its `next`, a function over its state. A
`for` loop calls it until it gives `None`, `undefined`:

```rust
pub struct Countdown {
    pub from: u32,
}

impl Iterator for Countdown {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.from == 0 {
            return None;
        }
        self.from -= 1;
        Some(self.from + 1)
    }
}

pub fn launch() -> Vec<u32> {
    Countdown { from: 3 }.collect()
}

pub fn sum_countdown() -> u32 {
    let mut total = 0;
    for n in (Countdown { from: 4 }) {
        total += n;
    }
    total
}

pub fn evens(limit: u32) -> impl Iterator<Item = u32> {
    (0..limit).filter(|n| n % 2 == 0)
}
```

```js
import { $iterator, $lazyRange } from "@rust-js/runtime";

function countdownIterator_next(countdown) {
  if (countdown.from === 0) {
    return;
  }
  countdown.from = (countdown.from - 1) >>> 0;
  return (countdown.from + 1) >>> 0;
}

export function launch() {
  return $iterator({ from: 3 }, countdownIterator_next).toArray();
}

export function sumCountdown() {
  let total = 0;
  const iter = { from: 4 };
  let n;
  while ((n = countdownIterator_next(iter)) != null) {
    total = (total + n) >>> 0;
  }
  return total;
}

export function evens(limit) {
  return $lazyRange(0, limit).filter((n) => n % 2 === 0);
}
```

- **A `for` loop of it, or a `while let Some(n) = it.next()`, is a loop of
  what its test makes**, `let n; while ((n = countdownIterator_next(iter))
  != null) { .. }`, as JS writes one; where a closure in the loop keeps
  `n`, it's `while (true) { const n = ..; if (n == null) { break; } .. }`,
  each turn's own `n`
  ([ADR 0860](../../decisions/0860-the-iterator-protocol-and-a-loop-of-what-its-test-makes.md)).
- **A chain of std's adapters is a JS iterator**, its helpers
  `filter`, `map`, `take` in Rust's order. A JS caller of `evens(7)`
  spreads it, `[...evens(7)]` being `[0, 2, 4, 6]`.
- **An iterator of your own, returned to JS as `impl Iterator`, is its
  struct**, `{ from: 3 }`, which JS can't `for..of` yet.
- **How each std iterator method reads** is
  [Collections](../for-js-developers/collections.md).
