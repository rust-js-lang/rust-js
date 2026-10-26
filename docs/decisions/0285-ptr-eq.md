# 0285. `std::ptr::eq` of JS objects is whether they're one

Status: Accepted, amended by [0407](0407-a-raw-pointer-is-a-handle-on-its-place.md):
`ptr::eq` of Rust values is their places', a pointer being a handle on
one. Extends [0111](0111-js-types-as-structs.md). Amended (2026-10-26):
of an item of an array that's an object and an object, whether it's the
item, `items[i] === r`, its index checked.

Case: A, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

`std::ptr::eq(a, b)` asks whether two references are to one place. A port
asks it of JS objects, whether a message came from this frame's window or
an element is the one it holds, and it was an error: the js crate told a
program to use `js::object::is` instead.

A JS object a binding gives, a struct of a `PhantomData<JsObject>` (ADR
0111), is only ever held by reference, and a reference to it is the JS
object: two are of one place exactly when they're one object. A Rust value
isn't so:

- **An unchanged copy is the object it was copied from**: `let b = a;` of
  a `Copy` struct no one changes is `const b = a;`, so `ptr::eq(&a, &b)`,
  `false` in Rust, would be `a === b`, `true`.
- **A number is its value**: two `&u32` of equal numbers at two places are
  `===`.

TypeScript has `===` and nothing of places; ReScript has `===` of any two
values, physical equality, with the same caveat for its unboxed values.

## Decision

**`std::ptr::eq(a, b)` of references to JS objects is `a === b`.** Of any
other type it's an error that says why: rust-js shares a copy no one
changes and writes a number as its value, so where a Rust value is has no
JS counterpart.

```rust
std::ptr::eq(a, b)  // a, b: &webapi::Element
```

```js
a === b
```

- The `*const T` rustc makes of each `&T`, `&raw const *r`, is `r`, the
  object; `r as *const T` is too.

## Why

- **Faithful where it can be**: for a JS object the answer is Rust's, and
  for a Rust value no answer would be.
- **An error over a wrong answer**: a program that compares places of
  Rust values learns so where it's written, not at run time.

## Alternatives

- **`===` of anything**, as ReScript's physical equality: `true` of two
  copies Rust holds apart.
- **Keeping every copy apart** so that addresses mean something: a copy
  of each value passed or bound, to answer a question few programs ask.

## Consequences

- `js::object::is` remains, for two JS objects seen as two Rust types.
- `Rc::ptr_eq` and `ptr::eq` of a `dyn` or a slice are still errors.

## Amendment: an item that's an object, and an object (2026-10-26)

React's hydration diff asks whether the node it skips to is its node's
only child:

```js
if (
  skipToNode !== node &&
  (node.children.length !== 1 || node.children[0] !== skipToNode)
) {
```

Its port, `std::ptr::eq(&node.children[0], skip_to_node)`, read
`!$ptrEq($ptr(node.children, 0), $ptrTo(skipToNode), 44)`: a handle on the
item's place, ADR 0407's, against the object's, which the runtime finds
the item's place of by the array the object was last seen in. And it was
wrong past the end: `$index(items, 0)`'s check, MIR's bounds check
deferred to where the item's read, was dropped where the handle was
made, so `ptr::eq(&items[0], r)` of no items was `false`, where Rust
panics `index out of bounds`.

- **A handle on an item an index check reaches keeps the check**:
  `$ptr(items, $at(items, i))`, `$at` throwing Rust's message past the
  end, as `xs[$at(xs, i)] = v` does (ADR 0056). A wrong answer fixed.
- **`ptr::eq` of an item that's an object, of no cell, and an object is
  `items[i] === r`**, `$index(items, i) === r` where its index is checked:
  a copy of what a pointer points to is a copy (`copies.rs`), so an
  object is at one place, and its item's place is the item's, as
  `$ptrTo` of two objects is already `a === b`. Of two items, of a value
  JS holds as itself, or of a type parameter's, which may be a number, it's
  the handles' `$ptrEq` as before. What each side is made of may be held
  where MIR makes it, `const _16 = $ptrTo(refs[0])` before a bounds check,
  so tidy asks it again where that's read in place (`same_object_in_place`,
  the pass `in place/same objects`): one rule, `pointers::same_object`, of
  both; a `$ptrEq`'s pointee has a size, so it's never a type parameter's.

Case ([0262](0262-when-rust-and-js-disagree.md)): A, the same answer;
the check, C, Rust's panic. Held by `test/corpus/ptr_eq_items_and_counts.rs`,
the port's `describeNode`, an item of an empty slice's panic, and each of
two items, run against native Rust; its snapshot, `node.children[0] !==
skipToNode` and `$index(items, 0) === node`, and `ptr_eq_through_refs.rs`'s,
`$index(refs, 0) === $index(points, 0)`; `lowering-collections.test.ts`'s
"ptr::eq of a type parameter's item is by its place"; and the mutations
`item-pointer-unchecked`, `item-and-object-by-handles`,
`item-and-object-of-any-pointee`, `item-and-object-unchecked` and
`same-objects-not-in-place`.
