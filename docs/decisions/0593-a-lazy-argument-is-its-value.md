# 0593. `use_state` takes what makes its first state; a lazy argument that does nothing is its value

Status: Accepted. Amends [0443](0443-a-value-a-caller-may-leave-out-is-an-option.md)'s
remaining `_with` forms of React's hooks: `use_state_with` is gone. Decided
with the user.

Case ([0262](0262-when-rust-and-js-disagree.md)): A for the lazy value: a
value made of nothing that does anything is the same made once or every
render. Function-typed state, which `use_state(f)` handed React as an
initializer, is now React's: a fix (case R).

## Context

React's `useState(initial)` takes a value or a function, `S | (() => S)`,
and calls a function for the first state. rust-js had two Rust functions,
`use_state(initial: T)` and `use_state_with(init)`, and the first gave
React a function as it was, which React called: state of a closure's type
was the closure's result. Rust can't say the union as one function, its
two impls overlapping. rescript-react's `useState` takes the function
alone, `(unit => 'state) => ..`, "the only way to safely have any type of
state"; scalajs-react's takes a by-name parameter, a thunk, always.

## Decision

**`use_state` takes what makes the first state, a closure or a function,
`impl FnOnce() -> T`**: `use_state(|| 0)`, `use_state(load_notes)`.

**A binding's parameter React calls once for its value is marked
`#[rust_js::lazy(name)]`, and a closure given it whose body does nothing
and gives no function is its value**: `use_state(|| 0)` is `useState(0)`,
`use_state(|| load().len())` stays `useState(() => load().length)`, and
`use_state(|| on_tick)` of a function stays a closure, as React would call
the function. A `lazy` that names no parameter is an error.

## Why

- **One function, of any state**, as rescript-react's.
- **The JS reads as React's**: `useState(0)` where the closure gives a
  constant, which is most of them.

## Alternatives

- **A union, `S | (() => S)`, as Scala.js's `js.|`**: two impls a closure
  satisfies both of, which rustc rejects; and a function state is what it
  can't tell apart.
- **A wrapper, `Lazy(|| ..)`**: every call spells the member it is.

## Consequences

- Each `use_state(x)` is `use_state(|| x)`: the guide, examples, tests and
  the playground. A call's state, `use_state(|| hash())`, is made on the
  first render only, `useState(() => hash())`, as React's docs advise.
- `use_reducer_with` and `use_optimistic_with` remain, React's other
  overloads of those hooks, for the user to decide.

## What holds it

- `test/react.test.ts`, "use_state's first state is what its closure
  gives": a constant, a function, a call, and a function's state.
- `test/diagnostics.test.ts`, "a lazy parameter that isn't there".
- Mutations: the closure kept, a call unwrapped, a function unwrapped, the
  attribute unchecked.

## Amendment: `use_state`'s initializer may borrow

`use_state` took `impl FnOnce() -> T + 'static`, so TodoMVC's
`use_state(|| todo.title.clone())`, of the component's own props, was
E0373, and the playground's browser test, the only one that compiled the
example, timed out on it. React calls the initializer during `useState`,
if at all, and keeps nothing of it: the bound is gone. The playground's
React examples, `react_counter` and `todomvc`, are native snapshots too,
`test/snapshots.test.ts`, which fails with E0373 under the old bound.
