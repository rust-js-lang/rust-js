# 0674. A struct literal's object lists its fields in the order written

Status: Accepted. Amends [0020](0020-structs-and-tuples.md), whose object
listed a struct's fields as declared, one shape of each type, and set a
field that did something aside where that reordered it;
[0250](0250-reference-base-spread.md), whose fields after the spread were
as declared too; [0204](0204-flattened-props.md), whose flattened field
was found by its place in the object; and [0203](0203-component-props-as-written.md),
whose props JSX reordered itself.

Case: N ([0262](0262-when-rust-and-js-disagree.md)): Rust has no order
of a struct's fields but its declaration's, which `{:?}` and serde keep;
JS's object has the order its literal writes, which only JS reads.

## Context

React's `shared/ConsolePatchingDev.js` gives `Object.defineProperties`
two objects of `console`'s methods, in two orders: `{ info: props, log:
props, .. }` when it disables them, `{ log: {...props, value: prevLog},
info: .. }` when it puts them back. The port writes one struct, `Logs`,
and two struct expressions, each in upstream's order:

```rust
&Logs {
    log: PropertyDescriptor { value: Some(&PREV_LOG.get()), ..props },
    info: PropertyDescriptor { value: Some(&PREV_INFO.get()), ..props },
    ..
}
```

and rust-js listed both in `Logs`'s declaration order, `info` first. MIR
gives an aggregate's operands as declared, so `adt_value` laid them out
so: an object's keys were the struct's, whatever the program wrote. And
where a field's value did something, as each of `P { c: f(), a: g() }`
does, the object, `{ a: g(), c: f() }`, couldn't evaluate them in Rust's
order, so the first written was a variable before it, `const value =
f();`.

## Decision

**A struct expression's object lists its fields in the order Rust
evaluates them: as written, then those `..base` fills, as declared.** A
spread of the base is before them, as JS's fields after a spread override
it, which is what `..base` means: it fills only the fields not written.

```rust
Logs { log: Some(prev), info: Some(prev + 1), warn: Some(prev + 2) }
Logs { warn: Some(1), log: Some(2), ..logs }
Options { size, ..Default::default() }
```

```js
({ log: prev, info: (prev + 1) >>> 0, warn: (prev + 2) >>> 0 });
({ ...logs, warn: 1, log: 2 });
({ size, name: "", tags: [] });
```

- **One owner of the order**, `aggregates.rs::written_order`: THIR's
  struct expression at the aggregate's place, of its ADT and variant, its
  fields as written. `mir_aggregate` reads the operands in that order,
  through `take_operands`, which keeps evaluation order (ADR 0364), and
  `adt_value` lists the object's fields so. Effects run as rustc runs them,
  each in place: `{ c: f(), b: String(g()), a: h() }`.
- **What's no struct expression keeps its declaration's order**: a tuple
  struct's or variant's array, `T { 1: a, 0: b }` too, which an array
  can't reorder; a value made by std, a constant's bits, a constructor as
  a value; and a place where two struct expressions of one type share a
  span, a macro's, where THIR can't say which.
- **A variant's tag is first**, `{ TAG: "Rect", h, w }`, and a flattened
  field's fields where it's written (ADR 0204), found by its name.
- **JSX's props are the object's, as it lists them**: ADR 0203's order,
  as written, then what a base gives, was JSX's own reordering of an
  object listed as declared, a second answer, and is now the object's.
  JSX puts children last, so a prop listed after them, what `{..base}`
  gives, that reads what children that do something change, `n={base.n}`
  of `{bump(&mut base)}`, is read before them, `const n = base.n;`, as
  one that does something already was. A flattened struct's attributes
  are found by name.
- **What Rust shows is as declared**: a derived `Debug`'s `{:?}`, serde's
  fields and a derived order are each written field by field, as Rust's
  are, whatever the object's order.
- **Destructuring already was as written**: `let P { c, a, .. } = p;` is
  `const { c, a } = p;`.

## Why

- **The JS a person writes of the Rust**: upstream's two objects of one
  shape, each in its own order, `log` first where React writes it first.
- **Rust's evaluation order, in place**: a field's value is evaluated
  where it's listed, so none is set aside in a variable to keep its order.
- **Only JS can see it**: `Object.keys`, `JSON.stringify` and
  `Object.defineProperties` read an object's order; Rust's `{:?}`, serde
  and a derived `Ord` read its fields by name, in the declaration's.

## Alternatives

- **The declaration's order everywhere**, ADR 0020's: what MIR gives, one
  order of a type's objects, so V8 gives them one hidden class. It's not
  what the program writes, and a field that does something written before
  one declared before it is set aside in a variable, as each is in a loop,
  `const value = step(i);`.
- **The base's fields first, `{ name: "", tags: [], size }`**, as the
  spread is: they're evaluated after the written ones, so a base that
  does something would be set aside; and `Pos { x: 9, ..pos }` reads as
  `{ x: 9, y: pos.y }`, as a person writes it.

## Consequences

- Two objects of one struct may have their keys in two orders, so V8 may
  give them two hidden classes, as JS written that way does.
- `test/corpus` snapshots where a struct expression is written out of its
  declaration's order list its fields as written.

## What holds it

- `test/corpus/struct_fields_as_written.rs`: fields that each record a
  step, written out of order, in a literal, an update of a moved base and
  one of a default, a variant's; destructured; shown by `{:?}` as
  declared, compared by a derived order; native and JS alike.
- `test/lowering-structs-enums.test.ts`, "a struct literal's keys are in
  the order written": `Object.keys` as written, an update's fields after
  its spread, a flattened field written first; "an update's default base
  is the defaults it reads": `{ size, name: "", tags: [] }`.
- `test/js-object.test.ts`, "console is a value, its functions read as
  values": React's `reenableLogs`, `log` first.
- `test/jsx-props.test.ts`, "JSX gives a component its props as
  written, and children that change the base first".
- Mutations `fields-as-declared`, `base-filled-first`,
  `flattened-field-by-place`, `fields-read-as-declared`,
  `spread-fields-as-declared`, `base-prop-read-after-children`,
  `jsx-flattened-attrs-by-place`.
