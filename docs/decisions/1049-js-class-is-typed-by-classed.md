# 1049. `js::class::<T>()` is typed by `T`'s `Classed`

Status: Accepted (rd410, P3, the user's decision, 2026-11-01). Amends
[0761](0761-a-rust-js-class-is-a-constructor-function-and-its-prototype.md)
and [0911](0911-a-bindings-class-is-the-global-typescript-names.md):
`js::class::<T>()` takes a `T: js::Classed`, and gives the `Class` its
`Constructor` types.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. The class is the
value the program names, `Int8Array`, and `new` of it makes what JS's
`new` makes. Nothing of Rust's differs.

## Context

React's Flight client hands each typed array's constructor, and
`DataView`'s, to one function, which constructs the view:

```js
case 79 /* "O" */:
  resolveTypedArray(response, id, buffer, chunk, Int8Array, 1, streamState);
  return;
..
function resolveTypedArray(response, id, buffer, lastChunk, constructor, bytesPerElement, streamState) {
  ..
  const view = new constructor(chunk.buffer, chunk.byteOffset, chunk.byteLength / bytesPerElement);
  resolveBuffer(response, id, view, streamState);
}
```

`js::class::<js::Int8Array>()` was `Int8Array`, but it was untyped, a
`&'static Class`, so `js::new` refused it. That refusal is right: an
untyped class held is made by its constructor's binding (ADR 1027). The
port therefore wrote eleven functions that each made one array,
`newInt8Array(buffer, byteOffset, length)`, and handed those in.

A class whose constructor is typed, `Class<fn(A, B) -> T>`, is what
`js::new` takes (ADR 1009). One type for every view's constructor lets
one function take any of them, which is what upstream does.

## Decision

**`js::class::<T>()` takes a `T: js::Classed`, a trait on the type, and
gives `&'static Class<T::Constructor>`.** The js crate types each typed
array's and `DataView`'s class as one view's constructor:

```rust
impl<B: 'static> Classed for Int8Array<B> {
    type Constructor = fn(&'static B, f64, f64) -> ArrayBufferView<B>;
}
```

So `js::class::<js::Int8Array>()` is a `&'static Class<fn(&'static
ArrayBuffer, f64, f64) -> ArrayBufferView>`, the same type for all eleven
arrays and `DataView`. `js::new(constructor, (buffer, offset, length))`
then prints `new constructor(..)`, and the port writes upstream's form:

```rust
type ViewConstructor = Class<fn(&'static ArrayBuffer, f64, f64) -> ArrayBufferView>;

fn resolve_typed_array(.., constructor: &'static ViewConstructor, bytes_per_element: f64, ..) {
    let view = js::new(constructor, (chunk.buffer(), chunk.byte_offset(), chunk.byte_length() / bytes_per_element));
    ..
}

resolve_typed_array(response, id, buffer, chunk, js::class::<js::Int8Array>(), 1.0, stream_state);
```

That signature form is ADR 1009's: what `new` makes, not a reference to
it.

Every other class's `Constructor` is `js::AnyConstructor`, held untyped
as before:

- **The binding crates implement `Classed` for each class they declare.**
  The js crate's are in `builtins/src/lib.rs` and its templates
  (`typed_arrays.ts`, `intl.ts`). webapi's generator gives one to each
  interface that has an interface object (none to a mixin's union). node
  gives one to `Buffer`. A module's type, `crypto#Hash`, has no global to
  name, so it gets none.
- **A program's `#[rust_js::class]` struct implements it in one line next
  to its `unsafe impl js::Defined`:**

  ```rust
  impl js::Classed for Component {
      type Constructor = js::AnyConstructor;
  }
  ```

  Without that line, rustc's error says to add it and writes it out:
  "`js::class::<Component>()` names no class: `Component` isn't
  `js::Classed`", noting `impl js::Classed for Component { type
  Constructor = js::AnyConstructor; }`.
- **A crate's class is held untyped.** A `Classed` of a signature on a
  `#[rust_js::class]` struct is refused. That signature is only the
  impl's word on what `new` takes, and the struct's constructor, `fn
  new(..)`, needn't take it.
- **An impl of a trait of types alone is no method**, so a class may
  implement one. The class rule (ADR 0761) refused every trait with items.
  Now it refuses only a trait with a method or a constant, because an
  associated type has no JS.

## Alternatives

- **A blanket impl that defaults to untyped, with a typed impl for each
  view.** These overlap, and choosing between them needs specialization,
  which stable Rust lacks. Associated-type defaults are unstable too. This
  is why a program's class needs its own line.
- **Typing by `T` itself, `Class<Of<T>>`.** Two classes would be two
  types. React's `if .. { js::class::<AbortController>() } else {
  js::class::<AbortControllerShim>() }`, and its `WeakMap`/`Map` choice,
  would then not type-check.
- **A second generic, `class::<T, S>()`.** Every `js::class::<T>()` would
  then need both arguments.
- **A per-type accessor, or `Class::of` of a constructor binding.** Either
  is a second way to name a class (the one-way rule). Each array's
  constructor would also give its own type, so no one function could take
  all of them.

## Consequences

- The Flight client hands `Int8Array` .. `DataView` as upstream does. Its
  eleven `newInt8Array`-style helpers are gone.
- Each program class that `js::class::<T>()` names has one more line.
  react-in-rust has six: Component, PureComponent, ComponentDummy,
  PingCache, AbortControllerShim and ReactPromise.
- A binding crate's new class needs its `Classed`. webapi's generator and
  the js crate's templates write it, so a regenerated crate keeps it.
- Tests: `test/js-constructors.test.ts` "a typed array's class is one
  view's constructor" (compiled, run in Node); `test/classes.test.ts`
  "what a class is refused says what to write" (the missing line, a typed
  crate class); and the class tests that now carry the line. Mutations:
  `class-typed-of-a-crate-struct`, `class-trait-of-types-refused`.
