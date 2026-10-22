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
