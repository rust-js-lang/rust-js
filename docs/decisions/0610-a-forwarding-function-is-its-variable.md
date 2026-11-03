# 0610. A function that only calls a module variable's function is that variable

Status: Accepted. Extends [0601](0601-a-module-variable-an-if-makes-is-a-let.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): A: the same function is
called with the same arguments.

## Context

React's scheduler picks its clock when it loads, and calls it
`getCurrentTime()`:

```js
let getCurrentTime;
if (typeof performance === 'object' && typeof performance.now === 'function') {
  const localPerformance = performance;
  getCurrentTime = () => localPerformance.now();
} else { .. }
```

In Rust that's a thread-local of a closure, and a function that calls it:

```rust
thread_local! {
    static GET_CURRENT_TIME: Box<dyn Fn() -> f64> = if .. { .. } else { .. };
}

pub fn get_current_time() -> f64 {
    GET_CURRENT_TIME.with(|get_current_time| get_current_time())
}
```

which was `let GET_CURRENT_TIME; ..` and `export function getCurrentTime()
{ return GET_CURRENT_TIME(); }`, a second name for one function.

## Decision

**A function that only calls a module variable's function with its
parameters is that variable, by the function's name**, exported where the
function was, `export { getCurrentTime };`, or `export const` of a
`const`. Each value the variable is given must be an arrow of Rust's,
which takes no more than its parameters, as the function did; one holding
a JS function, which a caller would give more, stays called by the
function.

Not where the module's load calls the function: a function declaration is
hoisted, so its load may call it before the variable is set.

## What holds it

- `test/lowering.test.ts`, "a function that only calls a module
  variable's function is the variable", with one the load calls; "a
  binding taken as a fn is its JS function", one of a JS function.
- Mutations: the function kept, one of a JS function taken, one the load
  calls taken, the variable unexported.

## Amendment: a private function only called is its variable, of any value

React's ReactAct picks how to wait a few microtasks where it loads, and
calls it by its name:

```js
const queueSeveralMicrotasks =
  typeof queueMicrotask === 'function'
    ? (callback: () => void) => {
        queueMicrotask(() => queueMicrotask(callback));
      }
    : queueMacrotask;
```

Its port's `thread_local! { static QUEUE_SEVERAL_MICROTASKS: fn(Box<dyn
Fn()>) = if .. { |callback| .. } else { queue_macrotask }; }` and `fn
queue_several_microtasks(callback) { QUEUE_SEVERAL_MICROTASKS.with(|f|
f(callback)); }`, the one natural Rust of a function value made where the
module loads, printed the `const` and a function calling it: one value is
another module's function, not an arrow, and the function gives none, a
statement of its call, not `return`.

**A function that isn't exported, by its own name or another (ADR 0596),
and that the crate's code only calls, by its parameters, is its variable
whatever the variable holds**, a JS function, another module's function or
an arrow; and of no value, `f(callback);`, as of one, `return f(n);`.
None of its callers gives it more than its parameters, so neither does
the variable's function get more; a function of a value gives the
variable's, and one of none gives `undefined`, which no caller reads: a
`()` is `undefined` where it's read, its call a statement before it.
One passed as a value, `const given = runNow;`, may be given more by
what calls it, and stays a function; an exported one, which JS may call
otherwise, keeps ADR 0610's arrows of Rust's.

Case N (ADR 0262): each call calls the same function with the same
arguments, at the same time; the module's load reads the variable before
nothing calls it, as before.

Tests: `lowering-forwarding`, a private function of no value of
another module's function one way, one a closure calls, and kept: one
passed as a value, one `pub`, and one a `pub use` exports by another
name. Mutations: each of the three left out, and the rule never taken.

## Amendment: a private function only called of a thread-local's field is that field (#533)

Case ([0262](0262-when-rust-and-js-disagree.md)): N. Each call calls the
same function with the same arguments, at the same time; only its
identity, `name` and `length` tell the function from the field's, and no
code reads them of a function only called.

alien-signals builds its system where its module loads and takes it
apart:

```ts
const { link, unlink, propagate, checkDirty, shallowPropagate } = createReactiveSystem({ .. });
```

Rust has no module-level `let` of a pattern, so its port keeps the system
in a thread-local and wraps each function, as idiomatic Rust does:

```rust
thread_local! {
    static SYSTEM: ReactiveSystem<NodeKind> = create_reactive_system(ReactiveSystemOptions { .. });
}

fn link(dep: &Node, sub: &Node, version: u32) {
    SYSTEM.with(|system| (system.link)(dep, sub, version));
}

fn unlink(link: &Rc<Link<NodeKind>>, sub: Option<&Node>) -> Option<Rc<Link<NodeKind>>> {
    SYSTEM.with(|system| (system.unlink)(link, sub))
}
```

which printed `const system = createReactiveSystem(..);` and five
functions, `function link(dep, sub, version) { system.link(dep, sub,
version); }`, a second name for each of the system's functions, as a
thread-local's function's caller was before this decision.

**A private function only called, by its parameters, that only calls a
function its module's constant's value holds, one that reads no `this`,
with its parameters, is that function: the value taken apart where it's
made, each field by its function's name, the functions gone, where only
those functions read the constant** (`prepare::forwarding::fields`):

```js
const { link, unlink, propagate, checkDirty, shallowPropagate } = createReactiveSystem({ .. });
```

`{ unlink: unlinkDep }` where the names differ. It's this decision's rule,
of a field: the function is the module's own, not exported nor a `pub
use`'s, and only called, by its parameters, so no caller gives it more nor
reads what a function of no value gives; not kept out of line (ADR 0649).
What's new is the field's:

- **It reads no `this`** (`Known::ThisUnread`, `Pointers::call_facts`):
  a field the crate fills with its own functions alone (#405), a `dyn
  Fn`'s or a `fn`'s. A binding's method,
  `epoch.getTime()`, reads its object as its `this`, and stays called
  through it, as does a function of an object JS made, which
  `may_read_this` calls through it.
- **It's the function the value was made with at each call**: only those
  functions read the constant, no export, so nothing writes its fields
  or sets it, each a read of it too, and reading each once where it's
  made reads what each call read. One another function reads, one
  another module imports, and one its module sets, a `let`, stay.
- **Its key is an identifier**, which a destructuring's key is unquoted:
  a field named `check-dirty` stays.
- **Where the module's load calls the function** makes no difference: a
  call before the value is made read it before it was made, as the
  function did, and the thread-locals made where they're first used read
  it after.

And of either rule now, **a call that leaves its last `undefined`s out,
`unlink(sub)` of `unlink(&sub, None)`, is one by its parameters**: it
gives what a call of them would, only `arguments.length` telling, which
is how rust-js writes every such call (ADR 0313). alien-signals' port
writes `unlink(sub)` as upstream does, which kept its function before.

Alternatives: a check of the wrappers in lowering, of THIR's `with`, its
closure's call of a field and rustc's `Freeze` (tried first): a second
owner of what this decision already asks of a function of the module's
own, and its JS's shape, which only this pass sees whole. A struct of the
functions as a `pub` thread-local exported, `#[rust_js::namespace]` (ADR
1037), is an export, which alien-signals' system isn't.

The port reads `const { link, unlink, propagate, checkDirty,
shallowPropagate } = createReactiveSystem({ .. });`, and upstream's suite
passes 202/202.

What holds it: `test/lowering-forwarding.test.ts`, "a private function
only called that only calls a thread-local's field is that field", run
(taken apart, a call of a `None` left out; kept: a binding's method, a
field of no identifier, a thread-local set, a call that converts what
it's given); the corpus case `thread_local_field_forwarders` against
native Rust (an `Rc`'s, a `Box`'s, a `fn`'s and a reference's fields, by
other names, of no value, read where the module loads; kept: read by
another function, named as a value, a method, a `Deref` of the crate's,
`#[inline(never)]`, exported, read by another module). Mutations
`forwarding-fields-*` and `forwarding-called-only-by-as-many`
(`prepare.ts`), `reads-no-this-of-own-fields-unseen`
(`analysis/fn_pointers.ts`),
`this-unread-unmarked` (`mir/calls.ts`).

## Amendment: a private thread-local of a namespace value read only for its methods is taken apart (#561)

Case ([0262](0262-when-rust-and-js-disagree.md)): N. Each call calls the
function it did, with the same arguments, at the same time.

React's noop Flight client makes one instance of the Flight client's
module, through its factory, and takes it apart once:

```js
const { createResponse, createStreamState, processBinaryChunk, getRoot, close } = ReactFlightClient({ .. });
```

Its port keeps the instance, a `#[rust_js::namespace]` value (ADR 1037),
in a private thread-local and calls through it,
`FLIGHT_CLIENT.with(|client| client.create_response(..))`, which printed
`const FLIGHT_CLIENT = ReactFlightClient({ .. });` and a member call at
each use, `FLIGHT_CLIENT.createResponse(..)`. Here there's no wrapping
function for #533's rule to take apart.

**A module's private `const`, each of whose reads is a call of its
member through it, of a function that reads no `this`
(`Known::ThisUnread`), is taken apart where it's made, each member by its
name, and each call is the function's alone**
(`prepare::forwarding::members`, after `fields`):

```js
const { createContainer, updateContainer } = createReconciler({ base: 100 });

export function noop() {
  return (createContainer(4) + updateContainer(1)) >>> 0;
}
```

- **A namespace's method through a value reads no `this`**: it's its
  module's function (ADR 1037), so its call is marked `ThisUnread` where
  it's lowered (`crate_called_copy`), as a field the crate fills with its
  own functions is (#533). A JS object's method, `epoch.getTime()`,
  isn't, and stays called through its object.
- **Only calls read it**: one also read as a value, `(kept.f(1), *kept)`,
  stays, and so does one set, a `let` its assignment reads, or one
  exported or a `pub use` names. So
  nothing writes its members, and reading each once where it's made
  reads what each call read.
- **Each member's name is new to the module**: one named as a function,
  a variable, a parameter or an import the module has already, which the
  call would then read, keeps the whole value. Of two such values with a
  member of one name, the first is taken apart and the second stays.

What holds it: `test/lowering-forwarding.test.ts`, "a private
thread-local of a namespace read only for its methods is taken apart
once", run in Node against a factory's instance (taken apart; kept: one
also read as a value, one the module sets, one whose member's name the
module's function has, the second of one name, a `Date`'s method). Mutations
`namespace-through-this-read` (`mir/calls.ts`) and `members-*`
(`prepare/forwarding.ts`).

## Amendment: only a namespace's methods are taken apart, not a struct's fields (#584)

Case ([0262](0262-when-rust-and-js-disagree.md)): N. Either form calls
the same function with the same arguments; this one keeps upstream's
names.

#561's rule asked `ThisUnread`, which a field the crate fills with its
own functions has too (#533). So React's ReactJSXElement, a `const`
struct of data read only to call its one field, was taken apart:

```js
const { react_stack_bottom_frame } = {
  react_stack_bottom_frame: (callStackForError) => callStackForError(),
};
```

where upstream keeps `createFakeCallStack` and calls the field through
it, a frame React's owner stacks look for by its name off the object.

**A call of a namespace's method through a value is marked
`Known::NamespaceMethod` as well as `ThisUnread` where it's lowered, and
`members` asks for it.** A struct's field, which only reads no `this`,
keeps its value's name and its field reads:

```js
const CREATE_FAKE_CALL_STACK = {
  reactStackBottomFrame: (callStackForError) => callStackForError(),
};
..
return CREATE_FAKE_CALL_STACK.reactStackBottomFrame(unknownOwner);
```

#533's `fields` still asks `ThisUnread`: a function of the module's own
that only passes on to such a field is that field.

What holds it: `test/lowering-forwarding.test.ts`, "a const struct of
fields read only to call them keeps its name". Mutations
`members-of-a-struct-of-fields` (`prepare/forwarding.ts`) and
`namespace-method-unmarked` (`mir/calls.ts`).
