# 0289. A `Copy` bound takes a copy only where a copy isn't the value

Status: Accepted. Amends [0052](0052-std-trait-impls.md): a `T: Copy`
copies with its dictionary only where it may be given a value whose copy
is another object. Amended: a caller whose value is its own copy, JS
among them, gives none, and the copy is the value.

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A `T: Copy` took a copy function, `TCopy`, and copied with it wherever a
`T` was used again (ADR 0052). Most callers give one whose copy is the
value itself: a number, text, a shared reference, an `Option` of one. For
them every call carried `{ copy: (value) => value }` and every use a
`TCopy.copy(x)`. react.dev's Preview debounces an `Option<&SandpackError>`
through `useDebounced<T: Copy>`, so `useDebounced(rawError)` became
`useDebounced(rawError, { copy: (value) => value })`, which no person
writes.

A copy matters only for an object changed in place: a `Copy` struct the
crate mutates (ADR 0052's changed types) is a new object each copy.

## Decision

**A type parameter's `Copy` bound takes a copy function only where it may
be given a value whose copy isn't the value: by a caller in the crate, by
a type parameter of the caller's own that is, or by a caller rust-js can't
see.**

```rust
fn twice<T: Copy>(value: T) -> (T, T) { (value, value) }
fn kept<T: Copy>(value: T) -> T { let copy = value; copy }

twice(name)                // twice(name), function twice(value)
kept(point)                // kept({ ...point }, { copy: (value) => ({ ...value }) })
```

- Its own copy: a shared reference, a number, `bool`, `char`, text, a
  function, a closure that changes nothing it captured, `()`, and an
  `Option` of one. Anything else, a type parameter's aside, gives one.
- A type parameter passed on, `relay<T: Copy>` calling `inner::<T>`, gives
  the callee one if the caller is given one.
- Every one is given a copy where callers aren't all seen: a trait's
  method and an impl's, called through dictionaries; a library's function
  its consumers can reach; a function of another crate.
- An impl's `T: Copy` counts as its method's.
- A trait impl's own `T: Copy` takes one too: its dictionary is given to
  callers rust-js can't see, as its methods are (found by rustc's
  `traits/conditional-dispatch.rs`).

## Why

- **It's the JavaScript a person writes**: `useDebounced(rawError)`.
- **It's the same program**: where no value changes in place, a copy and
  the value can't be told apart; where one may, its copy is still made.

## Consequences

- A function is given a copy function by one caller that needs it, and
  every caller then passes one, as with drops (ADR 0098). (Amended: a
  caller whose value is its own copy passes none.)

## Amendment: a caller whose value is its own copy gives none

Case ([0262](0262-when-rust-and-js-disagree.md)): None. A JS value, and
a Rust one this ADR calls its own copy, is the same value copied or not;
a value whose copy is another object is still copied.

React's port made `react-reconciler` use `react`'s crate, so `react`
became a library (ADR 0360), and each public `T: Copy` of a library takes
a copy (above). Its `createContext(defaultValue)` became
`createContext(defaultValue, TCopy)`, which read `TCopy.copy(defaultValue)`:
React's own JS, which calls `createContext(defaultValue)` as its
declarations say, `createContext<T>(defaultValue: T)`, threw, and 147 of
its tests failed.

**A copy function is given only for a value that may not be its own
copy; a function given none copies nothing.** Given one, it copies `x`
by it, `TCopy ? TCopy.copy(x) : x`; a caller whose value is its own copy,
JS or a Rust caller's number, gives none, left out at the end as a drop
is (ADRs 0098, 0313).

```rust
pub fn create_context<T: Copy>(default_value: T) -> Rc<Context<T>> { .. }

dep::create_context(7)               // createContext(7)
dep::create_context(Point { x: 1 })  // createContext({ x: 1 }, { copy: (value) => ({ ...value }) })
```

```js
export function createContext(defaultValue, TCopy) {
  return {
    currentValue: TCopy ? TCopy.copy(defaultValue) : defaultValue,
    currentValue2: TCopy ? TCopy.copy(defaultValue) : defaultValue,
  };
}
```

- JS calls a function as its declarations say, which list no copy or
  drop: in a program and a library alike, what JS gives is copied as the
  program's form copies it, not at all.
- A dictionary's `Copy` of a type that's its own copy, a supertrait's
  accessor say, is `undefined` too: whatever gives it, it's read the same.
- A crate's JS still changes when another crate starts using it, a
  program's becoming a library's (ADR 0360), but no JS caller's call of
  it does.

**Alternatives:**

- **A default, `TCopy = { copy: (value) => value }`:** each body is then
  `TCopy.copy(x)`, but only a parameter can have one; a copy an impl's
  dictionary or a supertrait's accessor gives would still be read as
  given: two forms where one reads every source.
- **A library writing its public functions as a program does**, no copy:
  wrong for a consumer whose `Point` it changes, as `copy_given` shows:
  `Cell::replace` hands it the object both cells hold.
- **`react` built as a program however it's used:** two forms of one
  crate again, which ADR 0360's amendment removed.

**Consequences:** `test/crates/pairs/copy_given` runs a library's
`create_context`, `twice` and `first` against native Rust, given a
`Point` its consumer changes, which they copy, and numbers, which they
don't, and calls each from JS as declared, `createContext(object)`,
getting the object itself. The corpus's in-crate callers that gave
`{ copy: (value) => value }` give nothing. A copy of what isn't a place,
`$someValue(value)`, is taken once, as every copy of one is, its `const`
named after the copy's parameter, `value`: `value$1` beside a `value`,
which `docs/readability.txt`'s renames count, blessed for it.
