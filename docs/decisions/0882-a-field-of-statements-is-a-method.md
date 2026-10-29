# 0882. A field that's a closure of statements is a method

Status: Accepted. Extends [0022](0022-closures.md)'s closures, an arrow,
to one an object's field holds. Amended below (the user's decision,
2026-10-26): a closure of statements a `let` binds is a function declared,
`function deleteChild(..) { .. }`. Amended again (the user's decision,
2026-10-27): in an object that has a method, each function field is one. Amended again (the user's rd96 decision, 2026-10-29): a function held as a value, a `let` of an `Rc` or a `Box` of a closure, is a `const` of its arrow.

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

## Amendment: a `let`'s closure of statements is a function declared

The user's decision (2026-10-26). React's ChildFiber makes its
reconcilers by a factory whose helpers are nested functions:

```js
function createChildReconciler(shouldTrackSideEffects) {
  function deleteChild(returnFiber, childToDelete) {
    if (!shouldTrackSideEffects) {
      return;
    }
    ..
  }
  ..
}
```

Its port's `let delete_child = |return_fiber, child_to_delete| { .. };`
printed `const deleteChild = (returnFiber, childToDelete) => { .. };`,
which ADR 0037's amendment for the factory left open.

**A `const` whose value is an arrow of statements is a function declared,
`function deleteChild(..) { .. }`, `async function` of an async one**
(`prepare::declared_functions`, on the completed tree, as `method` is a
field's). One of a value alone stays an arrow, `const double = (n) => n *
2`, as a field's does; so does one that reads `this` or `arguments`,
which a function has of its own, a class's method's closure reading its
`this` (ADR 0761).

Case N, as before: where hoisting can't change what runs. The binding is
a `const`, which nothing sets again; no read of its name comes before it
in its block, as JS's `const` throws there and the block's names are given
apart (ADR 0357), so a function declared from the block's start is read
where the `const` was; and a function and an arrow differ in `this`,
`arguments`, `new` and `prototype` alone, which no Rust closure reads but
the `this` that keeps one an arrow.

About 315 places in 129 of the committed JS outputs change, the corpus's
and the snapshots' re-blessed; held by `test/lowering-arrows.test.ts`, "a
let's closure of statements is a function declared", and the mutations
`let-closure-stays-arrow`, `let-closure-of-this-declared`,
`let-closure-of-a-value-declared` and `let-closure-async-unmarked`.

## Amendment: in an object with a method, each function field is one

The user's decision (2026-10-27). React's DEV dispatchers are objects of
methods, each written so, one of a value alone too:

```js
HooksDispatcherOnMountInDEV = {
  readContext<T>(context: ReactContext<T>): T {
    return readContext(context);
  },
  use,
  useCallback<T>(callback: T, deps: Array<mixed> | void | null): T {
    currentHookNameInDev = 'useCallback';
    ..
  },
  ..
};
```

Its port's `read_context: |context| read_context(context)` printed
`readContext: (context) => readContext(context)` beside methods of
statements: one object written in two forms.

**In an object that has a method, each field that's an arrow is a
method, of a value alone too**, `readContext(context) { return
readContext(context); }` (`prepare.rs`'s `method`, asked first whether
any field of the object is one of statements). An object of no method
keeps its arrows of a value, `{ double: (x) => x * 2 }`, the WebAssembly
import's; one that reads `this` stays an arrow wherever it is. Case N, as
before.

- **Alternatives**: every arrow in a field a method, which the decision
  rejected for the import's arrows; or an arrow of a value always, which
  leaves the dispatchers in two forms.
- Held by `test/lowering-arrows.test.ts`, "an object with a method has
  each function field a method", and the guide's `thenable`/`doubler`;
  mutations `field-method-beside-unseen` and `field-method-beside-any`.

## Amendment: a function held as a value is a `const` of its arrow

The user's decision for React's createEventHandle (rd96, 2026-10-29),
`const eventHandle = (..) => { .. }`, of the port's `let event_handle:
Rc<ReactDOMEventHandle> = Rc::new(..)`; ReactTestSelectors writes `const
commitHook = () => { .. }` of the port's `let commit_hook: Rc<dyn Fn()>`
too. Case N, as before.

**A `let` of an `Rc`, an `Arc` or a `Box` of a closure holds a function as
a value: a `const` of its arrow, as TypeScript writes a value of a
function's type, `const hook: Hook = () => { .. }`.** A closure the `let`
binds itself is a function declared, as before. MIR's lowering says so of
the variable's arrow, `js::Known::Valued`, by its type
(`recognition::holds_a_function`); `prepare::declared_functions` keeps it
a `const`.

The corpus's `dyn_fn_generic_closure_drops` and `rc_fn_ptr_eq`, the
playground's `stat` and the guide's `mountTransition` are `const`s of
their arrows. Held by `test/lowering-arrows.test.ts`'s "a let of an Rc or
a Box of a closure is a const of its arrow", run, and the mutations
`valued-arrow-unmarked` (`lower/mir/assign.ts`) and
`valued-arrow-declared` (`prepare.ts`).
