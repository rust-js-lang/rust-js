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
  - [A function's scope struct](#a-functions-scope-struct)
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
  One a pointer holds, `self: &Rc<Self>`, `self: Box<Self>` or `self:
  Pin<&mut Self>`, is named after its type too, `reply` of a `Reply`
  ([ADR 0047](../../decisions/0047-methods.md), amended).
- **The object is exported** where one of its methods is `pub`.

### A function's scope struct

A private struct whose one value a function makes, which only its own
methods and that function's closures hold, is that function's scope: its
methods are functions declared in it, after the value's `let`, calling
each other by name, and its fields are the function's variables, which
they read and write, as a JS function's nested functions read its
variables
([ADR 1043](../../decisions/1043-a-functions-scope-struct-is-its-scope.md), amended).

```rust
use std::cell::Cell;
use std::rc::Rc;

struct Reply {
    next_id: Cell<u32>,
}

impl Reply {
    fn part(self: &Rc<Self>, depth: u32) -> String {
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        match depth {
            0 => format!("${id}"),
            _ => format!("${id}:{}", self.part(depth - 1)),
        }
    }
}

pub fn process_reply(depth: u32) -> String {
    let reply = Rc::new(Reply { next_id: Cell::new(1) });
    reply.part(depth)
}
```

```js
export function processReply(depth) {
  let nextId = 1;

  function part(depth) {
    const id = nextId;
    nextId = (id + 1) >>> 0;
    if (depth === 0) {
      return `$${id}`;
    } else {
      const arg = part((depth - 1) >>> 0);
      return `$${id}:${arg}`;
    }
  }

  return part(depth);
}
```

- **Each call is a scope of its own**: each call of `process_reply` makes
  its value and its functions, as each call of a JS function does.
- **What makes one**: the struct isn't `pub`, has no const parameters and
  no trait impls; its methods take `&self`, `&mut self` or `self:
  &Rc<Self>`; the crate writes one literal of it, held by a `let` of the
  function's outermost block, itself or in an `Rc::new`; and it's given to
  no call but its methods', nor given back. Any other struct's methods are
  its type's object, as above.
- **A copy of a field named as the field is the field's variable**: `let
  controller = new(); self.controller.set(Some(controller));` is
  `controller = new AbortController();`, and `if let Some(controller) =
  self.controller.get()` reads `controller`, where each read of the copy
  is before anything that may set the field again, a call or a getter;
  past one, the copy stays, `const controller = controller$1;`.
- **Its function may be generic**, taking its callbacks as `impl Fn`, as
  processReply's `resolve` and `reject`: the struct and its methods are of
  no type parameter, so each of the function's lowerings declares them.
- **Its struct may be of type parameters**, alien-signals' `System<K>`
  of its callbacks, where each impl is of its own, `impl<K> System<K>`,
  and no method is given anything besides its arguments: no drop of a
  `K`, no bound's dictionary. Each call makes its one value, of its own
  types. A struct whose method drops a `K` or is given a bound's
  dictionary stays its type's object, given those as before; so does one
  whose method is `#[track_caller]`, whose calls give it their location.
- **A field is a `const`** where nothing writes it, and the parameter it's
  given where nothing assigns that. Where the JS reads the value whole, a
  tuple struct's array say, it keeps its object, `reply.nextId`.

alien-signals' `createReactiveSystem`, generic over its nodes' kind:

```rust
use std::cell::Cell;
use std::rc::Rc;

struct System<K> {
    notify: Box<dyn Fn(&K)>,
    count: Cell<u32>,
}

impl<K> System<K> {
    fn propagate(&self, node: &K) {
        self.count.set(self.count.get() + 1);
        (self.notify)(node);
    }
}

pub fn create_system<K: 'static>(notify: Box<dyn Fn(&K)>) -> Rc<dyn Fn(&K)> {
    let system = Rc::new(System {
        notify,
        count: Cell::new(0),
    });
    Rc::new(move |node| system.propagate(node))
}
```

```js
export function createSystem(notify) {
  let count = 0;

  function propagate(node) {
    count = (count + 1) >>> 0;
    notify(node);
  }

  return propagate;
}
```

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

```ts
// src/shape.d.ts
export interface Shape<Self> {
    area(self: Self): number;
    describe(self: Self): string;
}

export interface Circle {
    radius: number;
}

export interface Rect {
    width: number;
    height: number;
}

export const circleShape: Shape<Circle>;

export const rectShape: Shape<Rect>;
```

```ts
import type { Shape } from "./shape.js";

export function totalArea<S>(shapes: S[], SShape: Shape<S>): number;

export function circles(): number;
```

A Rust caller gives it, and so can JS and TypeScript, `totalArea([{ radius:
1 }], circleShape)`. The `.d.ts` declares the trait as the type of its
dictionaries, of the impl's type, `Self`, each method of the value first,
as the dictionary's function takes it; each dictionary of an impl its
module exports, as a `const` of it; and the function, with its dictionary
after its arguments
([ADR 1077](../../decisions/1077-a-function-js-cant-call-as-declared-isnt-declared.md),
amended). A TypeScript caller may give a dictionary of its own, `const
squareShape: Shape<Square> = { area: (s) => s.side ** 2, describe: (s) =>
"a square" }`, which generic code calls as it calls an impl's.

A trait is declared so where its dictionaries hold its methods alone, each
taking what its signature says. Not one:

- with a supertrait whose dictionary its own holds, `trait Labeled:
  Display`, or with an associated type or a constant;
- with a method of type parameters of its own, `fn show<T: Display>(&self,
  value: T)`, whose entry takes their dictionaries too; a `#[track_caller]`
  one, whose entry takes its caller's location; or one of a `&mut`, `fn
  bump(&mut self)`, which may take a box;
- of type parameters of its own, `trait Convert<T>`, a trait of classes,
  whose methods are its classes' prototypes', or a library's.

Nor is a generic impl's dictionary, `impl<T: Display> Shape for
Wrapper<T>`'s `wrapperShape(TDisplay)`, nor a std trait's: a `Display`
dictionary is the runtime's own. A function that takes one of those isn't
declared, and a note stands where it would be:

```ts
// `labels` isn't declared: its JS takes `L`'s `Labeled` dictionary after its arguments, which a JS caller can't give (ADR 1077).
```

A type parameter with no bound takes none, unless the function asks a fact
of it: `size_of::<T>()`, `align_of::<T>()`, `type_name::<T>()`, its
`TypeId` (a `T: 'static` made a `dyn Any` or compared by `TypeId::of`),
`needs_drop::<T>()`, the class of a map keyed by it, or an unstable sort of
its items. Then it takes that fact after its arguments, `size(TSize)`, which
only a Rust caller knows ([ADR 0145](../../decisions/0145-type-facts.md)).

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
  for (let i = 0; i < shapes.length; i++) {
    const shape = shapes[i];
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

```ts
import type { Shape } from "./shape.js";

export function largest(shapes: {
    value: unknown;
    impl: Shape<unknown>;
}[]): string;

export function mixed(): string;
```

Its `.d.ts`, last, declares each `dyn` of a trait declared as the type of
its dictionaries ([above](#generic-code-takes-a-dictionary)) as that pair,
of a value of some type, `unknown`, and that type's dictionary,
`Shape<unknown>`, as TypeScript writes a value of some type,
`Promise<unknown>`; a TypeScript caller gives `{ value: { radius: 1 },
impl: circleShape }`. A `dyn` of another trait, one not declared, is
`any`
([ADR 1077](../../decisions/1077-a-function-js-cant-call-as-declared-isnt-declared.md),
amended, #620).

A closure, `Box<dyn Fn(u32) -> u32>`, is a JS function, not a pair. A
field that's a closure of statements is a method, `then(resolve,
reject) { .. }`, as JS writes a thenable, and so is each function beside
it, as an object of methods is written; one of a value alone in an object
of no method stays an arrow, `{ double: (x) => x * 2 }`. A method and an
arrow differ only in their `this`, which a Rust closure never reads; one
that reads a class's `this` stays an arrow
([ADR 0882](../../decisions/0882-a-field-of-statements-is-a-method.md),
amended):

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

pub struct Doubler {
    pub double: Box<dyn Fn(u32) -> u32>,
}

pub fn doubler() -> Doubler {
    Doubler {
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
    double(n) {
      return (n * 2) >>> 0;
    },
  };
}

export function doubler() {
  return { double: (n) => (n * 2) >>> 0 };
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
- **A type inside itself is cloned, compared and ordered by a function of
  its module's**, one each, which every place in the module calls, and
  which calls itself for the ones inside: `cloneExpr(e)`, `cmpExpr(a, b)`,
  `partialCmpExpr(a, b)`, and `eqExpr(a, b)` where a part has your own
  `PartialEq` (otherwise `$eq` compares it). Of one made of a type
  parameter, `Tree<T>` in generic code, it's a function of the generic
  function, which reads the dictionaries that function is given.

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
- **A guard of a closure, scopeguard's `defer!` written by hand, is the
  closure's body in the `finally`**, where its `Drop` calls the closure
  once ([ADR 0608](../../decisions/0608-a-drop-is-its-body-at-each-drop.md#amended-2026-11-03-a-guard-of-a-closure-is-its-body-where-its-dropped)):

```rust
use std::cell::Cell;

thread_local! {
    static DEPTH: Cell<u32> = const { Cell::new(0) };
}

struct Defer<F: FnMut()>(F);

impl<F: FnMut()> Drop for Defer<F> {
    fn drop(&mut self) {
        (self.0)();
    }
}

pub fn nested(work: impl FnOnce() -> u32) -> u32 {
    let previous = DEPTH.get();
    DEPTH.set(previous + 1);
    let _restore = Defer(|| DEPTH.set(previous));
    work()
}
```

```js
let depth = 0;

export function nested(work) {
  const previous = depth;
  depth = (previous + 1) >>> 0;
  try {
    return work();
  } finally {
    depth = previous;
  }
}
```

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
  return Array.from($iterator({ from: 3 }, countdownIterator_next));
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
