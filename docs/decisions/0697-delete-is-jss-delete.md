# 0697. `delete` of a property is JS's `delete`, `js::delete`

Status: Accepted. Extends [0225](0225-unknown-values.md), whose
operators of a value of any shape were `js::get`, `js::set` and
`js::has`, and [0695](0695-new-of-a-function-value-is-jss-new.md)'s
`js::new`. Amended by [0776](0776-get-has-set-take-any-value.md): `js::delete`
takes any value, `undefined` too, which JS's `TypeError` is of.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: it's JS's
`delete`, which removes what JS removes and throws where JS throws.

## Context

React's `shared/ReactComponentStackFrame.js` puts a class's prototype
back as it was:

```js
if (prevProps !== undefined) {
  Object.defineProperty(fn.prototype, 'props', prevProps);
} else {
  delete fn.prototype.props;
}
```

The js crate had `reflect::delete_property(o, "props")`,
`Reflect.deleteProperty(o, "props")`, another function: nothing wrote
`delete`. TypeScript allows `delete` of an optional property, and of an
`any`'s.

## Decision

**`delete value[key]` is `js::delete(value, key)`, of a value of any
shape, as `js::get` reads one; it gives whether the property is gone.**

```rust
js::delete(prototype, "props");
let gone = js::delete(o, key);
```
```js
delete prototype.props;
const gone = delete o[key];
```

- **A key that's a name is a member**, `delete o.props`, as `js::get`'s
  is `o.props`; else `delete o[key]`.
- **A property JS can't delete, a frozen object's, throws a `TypeError`
  in strict mode**, which an ES module is: a panic, as a binding's throw
  is that doesn't say it throws.
- **Its JS is the operator, `UnaryOp::Delete`, which writes**: it has an
  effect, whatever it's of, so no pass moves a read of what it deletes
  before it, nor drops one whose result nothing reads.
  `UnaryOp::writes` is the one answer `has_effects` asks of an operator.
  The passes that ask whether an expression only reads, of a test or of
  what's read past a call, needn't: a `delete` is of a property, which
  each takes for what changes, and a second `delete` of one gives what
  the first did.

## Why

- **JS's operator, of what TypeScript allows it**: as `js::get`, `js::set`,
  `js::has` and `js::new` are of a value of any shape.
- **`Reflect.deleteProperty` is another function**, `Reflect`'s binding,
  which gives `false` where `delete` throws.

## Alternatives

- **`delete` only of `Reflect.deleteProperty`**: the JS upstream writes
  can't be written.
- **A statement of its own, `StmtKind::Delete`**: `delete` gives a value,
  which a program may read, `const gone = delete o[key]`.

## What holds it

- `test/js-object.test.ts`, "delete is JS's operator": of a name and of
  a key, a statement and a value, run, before a read of what it deletes,
  and of a frozen object, which throws.
- `test/js-object.test.ts`, "React's component stack frame is found as
  upstream finds it": `delete prototype.props;`, its guard's field its
  variable (ADR 0705).
- Mutations `js-delete-read`, `delete-has-no-effects`.
