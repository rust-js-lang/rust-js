# 0741. An `ArrayBufferView` is what each view derefs to, and `isView` takes any value

Status: Accepted. Amends [0283](0283-js-crate-covers-typescripts-es-library.md)'s
typed arrays, whose `buffer`, `byteOffset` and `byteLength` each bound
apart, and its `ArrayBuffer.isView`, which took a `&JsObject`.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: each is the JS
property, or the JS call, it was.

## Context

React's `shared/binaryToComparableString.js` takes any view of bytes:

```js
export default function binaryToComparableString(view: $ArrayBufferView): string {
  return String.fromCharCode.apply(String, new Uint8Array(view.buffer, view.byteOffset, view.byteLength));
}
```

and `shared/ReactPerformanceTrackProperties.js` asks `ArrayBuffer.isView(value)`
of any value. TypeScript has both: `interface ArrayBufferView<B> { buffer:
B; byteLength: number; byteOffset: number }`, which every typed array and
`DataView` is, and `isView(arg: any): arg is ArrayBufferView`. The js crate
had no type for a view, so the port took the one it could name, a
`&Uint8Array`, and `is_view` took a `&JsObject`, which a port classified a
value into first.

## Decision

- **`js::ArrayBufferView<B = ArrayBuffer>` is TypeScript's interface**, a
  JS object type of the three members, and **each typed array and
  `DataView` derefs to it**, as a class that extends another does
  ([inheritance is `Deref`](../guide/interop/bindings.md#inheritance-is-deref)):
  `view.buffer()` of a `&Uint8Array` is its `ArrayBufferView`'s, and a
  `&Uint8Array` goes where a `&ArrayBufferView` does. The three are bound
  once, on the view, and no more on each of the twelve types.
- **`array_buffer::is_view` takes any value, `Option<&Unknown>`**, as
  `array::is_array` does.

```rust
pub fn binary_to_comparable_string(view: &ArrayBufferView) -> String {
    let bytes = uint8_array::new_with_buffer(view.buffer(), Some(view.byte_offset()), Some(view.byte_length()));
```
```js
const bytes = new Uint8Array(view.buffer, view.byteOffset, view.byteLength);
```

`builtins/typed_arrays.ts` writes the view and each `Deref`;
`builtins/coverage.ts` counts a member of what a type derefs to as the
type's, as webapi's does, so `Uint8Array.buffer` is still bound.

## Why

- **TypeScript's own type**, of what upstream takes: no port narrows a
  view to one kind.
- **One binding of each member**: a twelfth copy of `get buffer` is a
  second way to read it once there's a view.

## Alternatives

- **A trait of the three, implemented by each view**: a function of any
  view would be generic, `V: ArrayBufferView`, given a dictionary, where
  a view is the object itself.
- **The trait as a marker, the members kept on each type**: twelve
  copies of each member, and a generic function couldn't call them.
- **`is_view(value: &T)` of any `Defined`**: `Array.isArray`'s is an
  `Option<&Unknown>`, one form of a test of any value.

## What holds it

- `test/js-builtins.test.ts`, "an ArrayBufferView is any typed array or
  DataView, and isView takes any value": a `Float64Array`, a `DataView` and
  a `subarray` given to one function, and `isView` of a view, an array,
  text and `undefined`, run.
- `test/builtins-coverage.test.ts`: each typed array's `buffer`,
  `byteOffset` and `byteLength` still bound.
