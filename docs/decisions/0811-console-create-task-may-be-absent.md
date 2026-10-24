# 0811. `console.createTask` is a property the host may lack

Status: Accepted. Amended: `#[rust_js::types]` is a TypeScript type, read
into the declaration model, one it can't read an error. Extends [0597](0597-whether-the-host-has-a-global-is-is-defined.md):
a member no TypeScript lib declares.

Case ([0262](0262-when-rust-and-js-disagree.md)): none: the program reads
what the host has, `None` where it hasn't it, as JS does.

## Context

React's DEV builds tag each element with a DevTools task, where the host
has V8's `console.createTask`:

```js
const createTask =
  __DEV__ && console.createTask
    ? console.createTask
    : () => null;
..
debugTask: console.createTask ? console.createTask('lazy()') : null,
```

Chrome and Node have it; Firefox, Safari, jsdom and Bun don't. No WebIDL
specifies it and no TypeScript lib declares it, so the webapi crate, made
of WebIDL as TypeScript's DOM is (ADR 0024), had no `console::create_task`.
React's Flow declares it on `console`, `createTask(label: string):
ConsoleTask`, with `interface ConsoleTask { run<T>(f: () => T): T }`.

ADR 0597 made `js::is_defined(binding)` the one way to ask whether a host
has a global TypeScript declares, as always there: reading a missing
global throws. A property of `console` doesn't: missing, it reads
`undefined`, as an optional property's does.

The user's decision: bind it, as a property that may be absent.

## Decision

**A member no TypeScript lib declares, which some hosts have, is declared
as an optional property, an `Option` of its value**, as TypeScript
declares one, `createTask?: (label: string) => ConsoleTask`: `None` where
the host hasn't it. The webapi crate's generator writes it by hand, in its
`EXTRA` members, now of a namespace too, and the type its function makes
in `TYPES_BY_HAND`, each typed as React's Flow declares it, and each
saying no TypeScript lib does:

```rust
// webapi
pub struct ConsoleTask(PhantomData<JsObject>);

impl ConsoleTask {
    #[cfg_attr(rust_js, rust_js::link_name = "run")]
    pub fn run<T>(&self, f: impl FnOnce() -> T) -> T { .. }
}

pub mod console {
    #[cfg_attr(rust_js, rust_js::link_name = "get console.createTask")]
    pub fn create_task() -> Option<&'static dyn Fn(&str) -> &'static ConsoleTask> { .. }
}
```

A program asks as React does, by the value:

```rust
pub fn debug_task() -> Option<&'static ConsoleTask> {
    console::create_task().map(|create_task| create_task("lazy()"))
}
```

```js
export function debugTask() {
  const createTask = console.createTask;
  return createTask ? createTask("lazy()") : undefined;
}
```

`ConsoleTask`'s `.d.ts` is Flow's interface, written out, `{ run<T>(f: ()
=> T): T }`, as there's no TypeScript name for it.

## Why

- **What JS reads**: `console.createTask`, `undefined` where the host
  hasn't it, which never throws, so it's the value's `Option`, as every
  optional property is, `Error.prepareStackTrace` among them.
- **ADR 0597's rule, a declaration as TypeScript's would be**: a global
  TypeScript declares as there is called as there, and asked of by
  `js::is_defined`; a member no lib declares, rust-js's crate declares as
  TypeScript declares what a host may lack, an optional property, not a
  method every host has. Each binding has one declaration, so a program
  asks of each one way.
- **React's JS**: `console.createTask ? console.createTask(..) : null`,
  not `typeof console !== "undefined" && typeof console.createTask !==
  "undefined"`.

## Alternatives

- **A method, `console::create_task(label)`, asked of by
  `js::is_defined(console::create_task)`**, as ADR 0597 asks of a
  TypeScript-declared member: it would declare every host has it, which
  no declaration says, and React's `console.createTask ? .. : ..` would be
  two `typeof`s.
- **`createTask`'s task an `Option` of the function's result too**, so
  React's `() => null` fallback has its type: a type looser than Flow's,
  which says it always makes one.

## Consequences

- React's `const createTask = __DEV__ && console.createTask ?
  console.createTask : () => null;` has no one Rust type for its two
  values, a `dyn Fn(&str) -> &ConsoleTask` and a closure that gives
  `None`. Its port holds the `Option` and calls it where it's there,
  `let createTask; if (__DEV__) { createTask = console.createTask; }`
  and `createTask ? createTask(name) : undefined`; a closure that forwards
  to it, `move |name| Some(create_task(name))`, is a function of its own,
  `(name) => createTask(name)`, not `console.createTask` itself.

## What holds it

- `test/webapi-bindings.test.ts`, "console.createTask is a function the
  host may lack, and its task runs what it's given": read once in DEV, a
  module's `let`, absent in Bun and present as a host gives it, its task
  running what it's given.
- `test/webapi-coverage.test.ts`: `webapi/src/lib.rs` is what
  `webapi/generate.ts` writes.
- The guide's [global scope](../guide/interop/global-scope.md#a-global-some-hosts-lack).

## Amendment: `#[rust_js::types]` is a TypeScript type, read

Case ([0262](0262-when-rust-and-js-disagree.md)): none at run time; a
declaration TypeScript reads as the Rust is.

`ConsoleTask`'s `#[rust_js::types = "{ run<T>(f: () => T): T }"]` was
declared `{ run<T>(f: () => T): T }>`: `#[rust_js::types]` had two
splitters, `written`'s and a type alias's, each taking a value apart at
its first `<` and its last `#`, and printing the rest as a name. A field
`#[rust_js::nullable] debug_task: Option<&'static ConsoleTask>` was
`debugTask?: { run<T>(f: () => T): T }> | null | undefined;`, which isn't
TypeScript, and an app's Prettier failed its whole build on it, as
react-in-rust's react crate's did. A union, `string | URL`, the value of
eighty of webapi's, came out right only as a name TypeScript's printer
wrote as it was given; `next`'s `string | import("next/image").StaticImageData`
did too.

**The value of `#[rust_js::types]` is a TypeScript type, which one reader,
`src/lower/declarations/written.rs`, reads into the declaration model's
parts**, printed by TypeScript as every other type is (ADR 0207):

| Written | Read |
|---|---|
| `Element`, `chart.js#Chart`, `NodeJS.TypedArray`, `Map<string, T>`, `ReactNode<>` | a `reference`, imported from its module, with its arguments |
| `string`, `null`, `unknown` | a `keyword` |
| `"auto"` | a `literal` |
| `string \| URL` | a `union` |
| `T[]`, `readonly T[]` | an `array` |
| `<T>(f: () => T) => T` | a `function`, with its own type parameters |
| `{ readonly a?: T; run<T>(f: () => T): T; [key: string]: T }` | an `object` of `property`, `method` and `index` members |

A name that's one of the Rust type's type parameters is the type it's
given, as before, anywhere in it now, but where a function or method has
a parameter of its name: `run<T>`'s `T` is its own. A name alone takes the
Rust type's arguments, `<>` none (ADR 0224); an alias's arguments end at
the first left to its default (ADR 0196). **One it doesn't read is a
compile error where it's written**, of a program, and of a binding crate
as `rust-js --rustc` builds it, saying what it reads: never a declaration
made of text it couldn't. `next`'s three `import("m").T` are `m#T`, the
one way a type names its module, which a union may now hold. What asked
of the text, whether it's `symbol`, `unknown` or `any`, or an object
literal, asks of what's read.

ConsoleTask's `.d.ts`, before and after:

```ts
debugTask?: { run<T>(f: () => T): T }> | null | undefined;
```

```ts
debugTask?: {
    run<T>(f: () => T): T;
} | null | undefined;
```

### Why

- **One rule, the one the attribute had already**: its values were
  TypeScript types, unions, `{ expire?: number }`, `{ [key: string]: T }`,
  each by a rule of its own or none. A reader of TypeScript's types, what
  bindings write, is one rule for all of them; an object type is one of
  them, as Flow's `ConsoleTask` is.
- **Correctness first**: a type rust-js couldn't read was printed as a
  name, which TypeScript's printer wrote however it read; a compile error
  that says what to write is the only other answer.
- **The model, not text**: what's read is the parts TypeScript prints, so
  the `.d.ts` is laid out as TypeScript lays out its own, an object's
  members each a line, and a later pass asks of its parts.

### Alternatives

- **A declared interface, `interface ConsoleTask { run<T>(f: () => T): T }`,
  that the attribute names, `ConsoleTask`**: no binding crate declares
  types of its own, so it would be a second place a binding says what its
  type is, a declaration each crate's `.d.ts` would need, of a crate that
  names it from another crate's; and every other object type
  `#[rust_js::types]` holds would still need reading.
- **TypeScript's own parser, in @rust-js/typescript**: the compiler would
  print a type it couldn't check, and a value's type parameters, which
  are the Rust type's arguments, would be substituted in text.
- **Keeping `import("m").T`**: a second way to name a module's type.
- **Intersections, `A & B`, and `typeof x`**: no binding writes one; one is
  the error saying what it reads, until a binding needs one.

### What holds it

- `test/declarations.test.ts`, "a binding's object type is declared as
  TypeScript has it": a nullable field of `&'static ConsoleTask` and a
  function's `Option<&'static ConsoleTask>`, checked by tsc under each
  tsconfig, and a type of `T` whose method has a `T` of its own.
- `test/declarations.test.ts`, "a #[rust_js::types] the compiler can't read
  is an error saying what it reads": of a program and of `rust-js --rustc`.
- Every shipped binding crate's build: `react/build.sh` (js and webapi),
  `node/build.sh`, and `next`'s, which each read every value they hold.
- `scripts/mutations/lower/declarations.ts`'s mutations of the reader and
  its use.
