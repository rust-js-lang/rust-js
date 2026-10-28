# 0994. A number a binding takes is TypeScript's `number`, an `f64`

Status: Accepted. Amends [0024](0024-web-crate.md), whose WebIDL integer
parameters were Rust integers, an `unsigned long` a `u32`; and
[0283](0283-js-crate-covers-typescripts-es-library.md), whose typed
arrays took their element's type and `u32`/`i32` lengths, indices and
offsets, and to `set` and `from` only a slice of their element. Follows
[0967](0967-an-object-spread-a-splice-and-a-set-by-any-key.md)'s
amendment, `js::array::splice` of numbers, as one rule.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's (the program's) behavior is kept: a number the program gives is
given to JS as it is, and JS converts it as its own callers' would be.

## Context

React's ports gave numbers to bindings typed narrower than TypeScript:

- SchedulerProfiling logs `ms * 1000` and a task's id into an
  `Int32Array`, `eventLog.set([..], eventLogIndex)`, and grows it,
  `newEventLog.set(eventLog)` (gaps rd1, rd2). `set` took `&[i32]`, so
  the port wrote `(ms * 1000.0) as i32`, `$f64ToInt(.., -2147483648,
  2147483647)`, which saturates where the store wraps (a profile past ~35
  minutes logs 2147483647, upstream the wrapped value); another typed
  array couldn't be given at all; and its sizes were `u32`, each sum `>>>
  0`.
- react-dom-bindings' selection code gives `range.setStart(node, offset)`
  and `selection.extend(node, offset)` JS numbers (gap rd26). An
  `unsigned long` was a `u32`, so the port wrote `offset as u32`,
  `$f64ToInt(offset, 0, 4294967295)`, saturating where WebIDL's
  `ToUint32` wraps.

The cast was the program saying something upstream doesn't: another
result for a number out of the integer's range.

## Decision

**A number a binding takes is TypeScript's `number`, an `f64`**: every
WebIDL number a function or a setter takes, not a `boolean` (a dictionary
member it's given too), and each length, index, offset and element a
typed array, an `ArrayBuffer` or a `DataView` takes, a `BigInt64Array`'s
element its `i64` (TypeScript's `bigint`). The host converts it as WebIDL
or ECMAScript says, as it does a JS caller's:

```rust
range.set_start(node, offset);
event_log.set(&[1.0, ms * 1000.0, id], Some(index));
new_event_log.set(event_log, None);
```

```js
range.setStart(node, offset);
eventLog.set([1, ms * 1000, id], index);
newEventLog.set(eventLog);
```

- **What it gives is its own type**, which the host's conversion made
  it: `length()` a `u32`, `node_type()` a `u16`, an element an `i32`.
  Rust's integer code, `0..view.length()`, a `match` of a constant,
  stays; giving one back is `f64::from(view.byte_offset())`, which is
  exact.
- **`set` and `from` take an `ArrayLike<T>`**, TypeScript's
  `ArrayLike<number>`: a slice, an array or a `Vec` of numbers each one
  `T` takes (`js::Takes`), or another typed array.
- **Written in `webapi/generate.ts`'s one place a number is typed**
  (`rustType`'s `NUMBERS`, of a parameter) and in
  `builtins/typed_arrays.ts`.

## Why

- **It's the type TypeScript gives, so the port's code is upstream's**:
  no cast where upstream has none, and none of a cast's other results.
- **One rule for every binding's number**, as `splice`'s was already,
  rather than an integer here and a number there.
- **It's exact**: every Rust number but a 64-bit integer converts to an
  `f64` exactly, so a program that had an integer gives the same value
  with `f64::from`.

## Alternatives

- **Results `f64` too, as TypeScript's are**: one type each way, but
  every integer a Rust program reads of the DOM, a length, a node's type,
  an index, cast back to count or match. Results are what the host
  converted already, so their integer type loses nothing.
- **A parameter generic over any number, `impl Into<f64>`**, so `item(0)`
  takes an integer literal: an optional one, `Option<impl Into<f64>>`,
  can't infer its type of `None`, the most common optional argument; and
  `splice`'s `f64` is the form already.
- **Keep the integer and convert in the lowering, `ToUint32` for a `u32`
  parameter**: the Rust would still have cast the `f64` first, which is
  where the value changed.

## Consequences

- An integer literal given to one is a float's, `labels.item(0.0)`,
  `int32_array::new(4.0)`, where it was `0`, `4`; its JS is the same.
- React's port's `binaryToComparableString` gives its view's offsets as
  `f64::from(view.byte_offset())`.
- `Atomics`' indices and values, and the js crate's other integer
  parameters (`String.prototype.charAt`'s, gap ch1), aren't yet changed:
  each follows this rule when it's ported.

## What holds it

- `test/js-collections.test.ts`, "a typed array takes numbers and another
  typed array as TypeScript types them": SchedulerProfiling's growth and
  log, run, `3e9` stored wrapped as upstream's store does.
- `test/js-builtins.test.ts`'s typed array, view and atomics tests, and
  `test/web_forms.rs`, with numbers given as `f64`s.
- The guide's libraries page (libraries/README.md).
- A binding's signature has no mutation: the tests that call it with an
  `f64`, a typed array or a `ms * 1000.0` don't compile without it.
