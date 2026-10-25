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

