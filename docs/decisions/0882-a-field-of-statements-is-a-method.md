# 0882. A field that's a closure of statements is a method

Status: Accepted. Extends [0022](0022-closures.md)'s closures, an arrow,
to one an object's field holds.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, no observable
difference: a method and an arrow differ in their `this` and their
`arguments`, and in nothing a Rust program sees. Neither is a
constructor, neither has a `prototype`, both are named after their key.
A Rust closure never reads `arguments`, and one that reads a class's
`this` (ADR 0761) stays an arrow.

## Context

React's ReactAct gives back a thenable whose `then` is a method:

```js
return {
  then(resolve, reject) {
    didAwaitActCall = true;
    ..
  },
};
```

Its port's `ActThenable { then: Box::new(move |resolve, reject| { .. }) }`
printed `then: (resolve, reject) => { .. }`, an arrow in a field, which
no person writes of an object's function of statements: react.dev's
`useImperativeHandle(ref, () => ({ focus() { .. } }))`, a `Proxy`'s
handler, `get(target, key) { .. }`, and React's ReactNoopUpdateQueue are
each methods.

## Decision

**A field whose value is an arrow of statements is a method, `name(a,
b) { .. }`, `async name() { .. }` of an async one. One of a value alone
stays an arrow, `double: (x) => x * 2`. One whose body reads `this` stays
an arrow.**

Preparation decides it (`src/prepare.rs`'s `method`), on the completed
tree, where the object's fields are as they'll print: `js::Prop::Method`
says so, and the printer writes it. It's every object literal's, a
struct's, a dictionary's `$drop` and a `$unionField` table's alike, one
rule for a field of a function. A JSX attribute isn't an object's field.

A field named `__proto__` is a method as any other: only `__proto__:
value`, the colon form, sets the prototype.

## Why

- **It's how JS writes an object's function of statements**: ReactAct,
  react.dev and a `Proxy`'s handler each do.
- **A value alone reads as it is**: a WebAssembly import, `{ env: {
  double: (x) => x * 2 } }`, is written so, and its method, `double(x) {
  return x * 2; }`, three lines.
- **`this` is the one difference**, and a closure that reads it is one a
  class's method made (ADR 0761), whose `this` is the instance: as a
  method it would be the object it's in.

## Alternatives

- **Every arrow in a field a method**, of a value too: the WebAssembly
  import and every dictionary's `consume: (arg0) => ..` three lines each.
- **Only a Rust struct literal's**: the object doesn't say what made it,
  and a dictionary's `$drop` and a struct's `then` are each an object's
  function, written alike.
- **An arrow everywhere, as before**: ReactAct's port keeps a difference
  from upstream in every thenable.

## Consequences

- `{ then(resolve, reject) { .. } }`, `{ init() { .. } }` of a
  `LazyCell`, `$drop(wrap) { .. }` of a dictionary.
- Tests: `classes` has a field's closure that reads `this`, an arrow, and
  one beside it that doesn't, a method; `js-builtins` a `Proxy`'s handler,
  `get(target, key) { .. }`; the `async` snapshot a WebAssembly import's
  arrows of a value. Mutations: the rule left out, `this` not looked for,
  an arrow of a value made a method, and the printer's method flag.

## Amendment: a method's comments and blank line are before it

React's ReactNoopUpdateQueue writes a comment before each of its
functions, and a blank line between them. Its port's `static
REACT_NOOP_UPDATE_QUEUE: Updater = Updater { .. }` printed the comment
before `enqueueForceUpdate` inside its body, and lost the blank line:

```js
  isMounted: (publicInstance) => false,
  enqueueForceUpdate(publicInstance, callback, callerName) {
    // Forces an update. ..
```

A comment before an object's field is before its value's start (ADR
0708), which the printer asked of a `Field` alone: a method's value
start wasn't one, so its comments went to the next start there was, its
body's first statement, and its blank line with them. A method's is now
a field's start as any other's:

```js
  isMounted: (publicInstance) => false,

  // Forces an update. ..
  enqueueForceUpdate(publicInstance, callback, callerName) {
```

Test: `lowering-comments`'s ReactNoopUpdateQueue's fields, a comment and
a blank line before each. Mutation: a method's start left out.

