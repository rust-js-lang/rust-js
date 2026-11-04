# 1082. A JS function of any signature is a `js::Function`

Status: Accepted (2026-11-04, the user's decision, #589). Amends
[0700](0700-typescripts-function-gives-any-value.md), whose TypeScript
`Function` was `&dyn Fn() -> Option<&'static Unknown>`; extends
[0740](0740-a-functions-members-are-read-of-any-function.md), whose
`AnyFunction` it is one of.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: each value is the JS
function it was, and the JS of each use is what it was; only the types a
`.d.ts` declares change, to TypeScript's own.

## Context

react-noop-renderer's flight-modules stores the Flight tests' render
functions, of any signature, and gives them back:

```js
const modules: Map<string, Function> = new Map();
export function saveModule(render: Function): string { .. }
export function readModule(idx: string): Function { return modules.get(idx); }
```

ADR 0700 made TypeScript's `Function` `&dyn Fn() -> Option<&'static
Unknown>`: a function called of nothing that gives any value. That reads a
call well, but as a type stored or exported it's a zero-argument
function's, `() => unknown | null | undefined`, narrower than `Function`:
a render function of props isn't one. The port held them as `&'static
js::Unknown` instead, `saveModule(render: unknown)`, looser than upstream
(rd724). `js::AnyFunction<M>` (ADR 0740) is a bound, `F: AnyFunction<M> +
?Sized`, that nothing can be stored as.

## Decision

**A JS function of any signature is one type, `js::Function`: a
`PhantomData<JsObject>` struct, `#[rust_js::types = "Function"]`,
`unsafe impl Defined`.** Every binding of TypeScript's `Function` takes
and gives it, and `Kind::Function` holds it, so a function read is one
given, kept or compared, as ADR 0700 wanted, with a type that says what
it is:

```rust
thread_local! {
    static MODULES: RefCell<HashMap<String, &'static Function>> = RefCell::new(HashMap::new());
}

pub fn save_module(render: &'static Function) -> String { .. }
pub fn read_module(idx: &str) -> Option<&'static Function> { .. }
```
```ts
export function saveModule(render: Function): string;
export function readModule(idx: string): Function | null | undefined;
```

- **Made of any function by `js::Function::of(&f)`**, `link_name =
  "this"`, as `Class::of` is: generic over `F: AnyFunction<M> + ?Sized`,
  a function item, a closure, a `fn` pointer, a `dyn Fn` or a binding's
  function, the function itself in JS. `Function::of(&twice)` is `twice`.
- **It implements `AnyFunction<Function>`**, and so does `&Function`, so
  `function::call`, `apply`, `bind`, `name`, `length`, `to_string` and
  `object` take it; `Class::of(f)` of one is a constructor of no
  parameters that makes any value, as of 0700's type.
- **Called of nothing, `f()`, it's `f()` (the user's choice A')**: a
  `Function` derefs to `dyn Fn() -> Option<&'static Unknown>`, which the
  compiler, as for every JS object type's `Deref`, compiles to the object
  itself (`is_js_object_deref`). React's `initialStateInitializer()`,
  `refCleanup()` and `onLoadStart()` are each upstream's call. Called of
  arguments, or of a `this`, it's `function::call(f, this, args)`.
- **Its `typeof` is `"function"`**: `classify`'s `Kind::Function` of it
  is told by `typeof value === "function"`, as TypeScript's global
  `Function` is every function's type in any realm, not `instanceof
  Function`, which a frame's function isn't
  (`declarations::written::Written::is_function`).

Every binding of TypeScript's `Function` moved to it, in one cutover
(the user's choice: all of them, not `Kind` alone):

| Binding | Was | Is |
|---|---|---|
| `js::Kind::Function` | `&'a dyn Fn() -> Option<&'static Unknown>` | `&'a Function` |
| `reflect::apply(target, ..)` | `&dyn Fn() -> ..` | `&Function` |
| `reflect::construct(target, .., new_target)` | `&dyn Fn() -> ..`, `Option<&dyn Fn() -> ..>` | `&Function`, `Option<&Function>` |
| the `WeakKey` of a function | `dyn Fn() -> ..` | `Function` |
| `function::BIND` | `&'static dyn Fn() -> ..` | `&'static Function` |
| `function::bind(f, this, &args)` of a slice or `Vec` | of `f`'s marker `fn() -> ..`, a `&dyn Fn() -> ..` | of a `Function`, a `&Function` |
| `js::method((value, "key"))` | `&'static dyn Fn() -> ..` | `&'static Function` |
| node's `CallSite.getFunction()` | `Option<&'static dyn Fn() -> ..>` | `Option<&'static Function>` |
| node's `Error.captureStackTrace(.., constructorOpt?)` | `Option<&dyn Fn() -> ..>` | `Option<&Function>` |
| webapi's 2 `QueuingStrategy.size`, WebIDL's `Function` | `&'static dyn Fn() -> ..` | `&'static js::Function` |

`PropertyDescriptor.get` stays `&dyn Fn() -> Option<&'static Unknown>`:
TypeScript types it `get?(): any`, a getter's own signature, not
`Function`. So does every typed function a program holds, React's
`() => mixed` resource say: `dyn Fn() -> Option<&'static Unknown>` is
Rust's function of no parameters that gives any value, as it always was.

## Why

- **TypeScript's own type**: a stored or exported function of any
  signature is declared `Function`, as upstream's is, neither narrower
  (`() => unknown`) nor looser (`unknown`).
- **One way**: one type of a function of any signature, held, given and
  returned; `AnyFunction` stays the bound a generic binding takes, which
  `Function` is one of.
- **The JS is upstream's**: `f()` stays a call of nothing, by `Deref`,
  where `function::call(f, None, ())` would be `f.call(undefined)`, a read
  of `f.call` upstream never makes.

## Alternatives

- **`f()` written `function::call(f, None, ())`** (the design as first
  drawn): prints `f.call(undefined)`, which reads `f.call` and differs from
  upstream's `f()`; printing it `f()` would drop that read, a change in
  results (ADR 0262).
- **`Kind::Function` alone moved**: two types of TypeScript's `Function`,
  the bindings' `dyn Fn` and `Kind`'s `Function`, and a value of one not
  given where the other is taken.
- **A generic binding parameter, `&F` of any function**, for what a
  binding takes: ADR 0700's rejected alternative still, an `Option<&F>`'s
  `None` can't infer `F`, and nothing is stored as one.
- **A `Deref` to `JsObject`**, as `Arguments` has: a second way to the
  object a function is, which `function::object(f)` already gives.

## Consequences

- No deprecation: a program that named `dyn Fn() -> Option<&'static
  Unknown>` where a binding of `Function` takes or gives one names
  `js::Function`, or gives `js::Function::of(f)`; a `WeakMap<dyn Fn() ->
  .., V>` is a `WeakMap<js::Function, V>`. Its JS is what it was; its
  `.d.ts` says `Function`.
- A `&Function` isn't given where a `&dyn Fn() -> ..` is taken by deref
  coercion: rustc tries the unsizing first and stops at it. `f()` goes
  through the `Deref`; a holder of what a binding of `Function` gives is a
  `&Function`.
- React's port moved in the same window as rust-js's pin, by a script of
  its edits, so its build pins no rust-js between.

## What holds it

- `test/js-object.test.ts`, "TypeScript's Function gives any value": a
  `Kind::Function` called, `value()`, what it gives read; `Reflect.apply`
  of a `Function`; a Rust `fn` given as one; run. "React's component stack
  frame is found as upstream finds it": a `WeakMap<Function, String>`,
  `Reflect.construct` of `Function::of(&fake)`, run.
- `test/js-functions.test.ts`, "a JS function of any signature is a
  js::Function": one stored in a map, returned, made of a function item
  and a closure, its `.d.ts` `Function`, run.
- `test/js-builtins.test.ts`, "a weak map is keyed by what js::Kind
  holds", and `test/node-bindings.test.ts`, "V8's stack trace hook is
  saved, set and restored", of the new type.
- `test/webapi-coverage.test.ts`, "the webapi crate is what its generator
  writes".
- Mutations `function-type-is-a-class` and `untagged-function-type-a-class`
  (`Function` a class `instanceof Function` would test, refused), and
  `function-type-is-any-reference` (every named type a function's).
- The guide's [JS values](../guide/interop/types.md#js-values-jsobject-and-unknown).
