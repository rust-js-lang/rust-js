# 0909. A type parameter's value is any value: `js::any_of`

Status: Accepted. Extends [0225](0225-unknown-values.md)'s `js::Unknown`
and [0673](0673-a-function-is-a-value-of-any-shape.md)'s `js::unknown_of`,
which take a value that's always there, to a type parameter's value,
which may be `undefined`.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. The value is given to
JS as it is and `js::cast` gives it back as it was; nothing a program
reads of it differs. Where it would, a cast to another type than it was
given as, the program has broken `js::cast`'s contract, which is
`unsafe`, as JS's own `Array<any>` is unchecked.

## Context

React's `ReactFiberStack` keeps the value each cursor had before a push,
of every cursor's type, in one untyped array, and gives it back on the
pop:

```js
const valueStack: Array<any> = [];
let index = -1;

function push<T>(cursor: StackCursor<T>, value: T, fiber: Fiber): void {
  index++;
  valueStack[index] = cursor.current;
  ..
  cursor.current = value;
}
```

Rust's way to keep values of erased types is `Box<dyn Any>` and a
`downcast`. The port wrote it, and it printed what `dyn Any` is (ADR
0331), a pair of the value and its type's dictionary, the `TypeId` its
generic function is given as a parameter of its own:

```js
export function push(cursor, value, fiber, TId) {
  ..
  const previous = { value: old, impl: { typeId: () => TId } };
```

so every JS caller of upstream's, `push(cursor, value, fiber)`, gave it
no `TId`, and `pop`'s `$unwrapOk(any.impl.typeId() === TId ? ..)` failed.
The `TypeId` check can't be left out: a pop of another cursor's value is
a panic in Rust, and nothing in a library function proves it never
happens.

Upstream's array is JS's `Array<any>`, which rust-js has a type of,
`Vec<Option<&'static Unknown>>`, and `js::cast::<T>` gives a `T` back of
one. Nothing put a type parameter's value in: `js::unknown_of` takes an
`AnyShape`, a value that's never `undefined`, and a `T` may be an
`Option`'s `None`.

## Decision

**`js::any_of(value)` is a type parameter's value as any JS value, an
`Option<&'static Unknown>`, the value itself.** `js::cast` gives it back.

```rust
thread_local! {
    static VALUE_STACK: RefCell<Vec<Option<&'static Unknown>>> = const { RefCell::new(Vec::new()) };
    static INDEX: Cell<i32> = const { Cell::new(-1) };
}

pub fn pop<T>(cursor: &StackCursor<T>, fiber: &Fiber) {
    ..
    cursor.current.replace(unsafe { js::cast(VALUE_STACK.with_borrow(|value_stack| value_stack[index])) });
    VALUE_STACK.with_borrow_mut(|value_stack| value_stack[index] = None);
    INDEX.set(INDEX.get() - 1);
}

pub fn push<T: 'static>(cursor: &StackCursor<T>, value: T, fiber: &Fiber) {
    INDEX.set(INDEX.get() + 1);
    let index = INDEX.get() as usize;
    let previous = js::any_of(cursor.current.replace(value));
    ..
```

```js
export function pop(cursor, fiber) {
  ..
  cursor.current = $index(valueStack, index$1);
  valueStack[$at(valueStack, index$1)] = undefined;
  index = (index - 1) | 0;
}

export function push(cursor, value, fiber) {
  index = (index + 1) | 0;
  const index$1 = index >>> 0;
  const previous = cursor.current;
  cursor.current = value;
  ..
```

- **It's a binding, `this`**, as `js::unknown_of` is: it's given its
  value and gives it, moved, kept as long as JS has it, as a `Box::leak`
  would be; `js::cast` gives it back, as a `Box::from_raw` would, once.
- **`T: 'static`**, as `dyn Any`'s is: what's kept outlives the borrow it
  was made of, which a `&'a` in a `T` would not.
- **Of a type that's known, it's refused**, with what to write: a value
  that's always there is `js::unknown_of(value)`'s, an `&Unknown`, and an
  `Option` of one `value.map(js::unknown_of)`. So there's one conversion
  of each value, and `any_of` is the one of a type rust-js doesn't know.

## Why

- **It's upstream's JS**: the value itself, and `push(cursor, value,
  fiber)` given only its arguments, as every JS caller calls it.
- **It's `unsafe` where upstream's is unchecked**: `js::cast`'s contract
  is what `Array<any>` asks of each `valueStack[index]` read, a value of
  the cursor's own type, which balanced pushes and pops keep. Rust's
  `Box<dyn Any>` checks what JS doesn't, which upstream's callers don't
  give it the means to.
- **It's the JS boundary's own type**: an untyped JS array is
  `Vec<Option<&Unknown>>`, as a binding's `Array<any>` is; only putting a
  `T` in was missing.

## Alternatives

- **`Box<dyn Any>` printed as its value where its downcast is known**:
  a library function can't know its callers pop what they pushed, and
  leaving the check out is a wrong answer where they don't, which Rust
  panics of. Rejected for correctness (ADR 0262).
- **`js::unknown_of` of any `T`**: its `&Unknown` is never `undefined`,
  which a `None` is; an `Option` result breaks every caller that has a
  value that's always there.
- **A generic of `T: IntoUnknown`**: `IntoUnknown` is every type, so the
  bound says nothing, and something still has to give the `T` as a
  value.
- **A stack of each cursor's own type**, `cursor.stack: Vec<T>`: another
  program than upstream's, whose one `index` its DEV check reads of
  `fiberStack`.

## Consequences

- What's printed beside the value stays Rust's: `index as usize` is
  `index >>> 0`, a `Vec`'s index is checked, `$index(..)`, and its
  `None` is `undefined` where upstream writes `null`, which nothing reads.
  `push`'s `replace` reads `cursor.current` before it's set, then stores
  it, where upstream stores it first.

## What holds it

- `test/js-object.test.ts`, "a type parameter's value is any value, kept
  and given back as itself": React's value stack of three cursors, an
  `Rc` of a struct's, a number's and an `Option<Option<u32>>`'s, pushed
  and popped in Node, its `push(cursor, value, fiber)` with no `TId` and
  no pair; and `js::any_of` of a `u32` and of an `Option<&str>` refused.
- Mutation `any-of-of-a-known-type`.
