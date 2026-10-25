# Hooks

React's hooks are the `react` crate's functions, in snake_case:
`use_state` is `useState`. Each compiles to the hook call itself, so React,
its DevTools and its rules see ordinary React.

## What every hook shares

- **What a hook gives back is `&'static T`**: React keeps it, and it's
  read-only, as React's state is. A shared reference is `Copy`, so every
  handler can `move` it in, and `*count` reads it.
- **What a hook gives to call**, a setter, a dispatch, a ref, is `Copy`
  too, and the function itself in JS.
- **A hook whose argument is optional** takes an `Option`.
- **`use_state` takes what makes the first state**, a closure or a
  function, called on the first render only: `use_state(|| 0)` is
  `useState(0)`, and `use_state(load_notes)` is `useState(loadNotes)`. A
  closure whose body does nothing is its value in JS
  ([ADR 0593](../../decisions/0593-a-lazy-argument-is-its-value.md)).
- **Call hooks at the top level** of a component or a custom hook, as
  React's rules say. rust-js doesn't check this; React does, in
  development.

## `use_state`

```rust
use react::{JSX, jsx, use_state};

pub fn Counter() -> JSX::Element {
    let (count, set_count) = use_state(|| 0);
    jsx! {
        <>
            <p>{"Clicked "}{count}{" times"}</p>
            <button onClick={move |_| set_count.update(|n| n + 1)}>{"+1"}</button>
            <button onClick={move |_| set_count.set(0)}>{"Reset"}</button>
        </>
    }
}
```

```jsx
import { useState } from "react";

export function Counter() {
  const [count, setCount] = useState(0);
  return (
    <>
      <p>Clicked {count} times</p>
      <button onClick={() => setCount((n) => (n + 1) | 0)}>+1</button>
      <button onClick={() => setCount(0)}>Reset</button>
    </>
  );
}
```

- `set_count.set(value)` is `setCount(value)`.
- `set_count.update(|n| n + 1)` is `setCount((n) => n + 1)`, the next
  state from the latest one, which the closure only reads.
- The setter where a prop wants a function, `&dyn Fn(T)`, is `&**set_count`,
  `setCount` itself.
- `use_state(load_notes)` is `useState(loadNotes)`: the first state,
  computed on the first render only. State that is a function is made by
  a closure that gives it, `use_state(|| on_tick)`, which stays one in JS,
  as React would otherwise call it for the state.

Ownership is how React's rule looks in Rust: a handler can't mutate the
state it was given, which in JS would change React's copy and not render.
Make a new value and `set` it.

## `use_effect`

`use_effect(effect, deps)` runs `effect` after a render. `deps` says when:

| Rust | JS | Runs |
|---|---|---|
| `Some(&(count, name))` | `[count, name]` | when one changed |
| `Some(&())` | `[]` | once, after mounting |
| `None` | none | after every render |

The effect returns nothing, or the closure that cleans it up:

```rust
use react::webapi::window;
use react::{JSX, jsx, use_effect, use_state};

pub fn Title() -> JSX::Element {
    let (count, set_count) = use_state(|| 0);
    use_effect(
        move || window.document().set_title(&format!("{count} clicks")),
        Some(&(count,)),
    );
    jsx! { <button onClick={move |_| set_count.update(|n| n + 1)}>{count}</button> }
}

pub fn Ticker() -> JSX::Element {
    let (ticks, set_ticks) = use_state(|| 0);
    use_effect(
        move || {
            let id = js::set_interval(Box::new(move || set_ticks.update(|n| n + 1)), 1000.0);
            move || js::clear_interval(Some(id))
        },
        Some(&()),
    );
    jsx! { <span>{ticks}</span> }
}

pub fn Logger() -> JSX::Element {
    use_effect(|| println!("rendered"), None);
    jsx! { <span /> }
}
```

```jsx
import { useEffect, useState } from "react";

export function Title() {
  const [count, setCount] = useState(0);
  useEffect(() => {
    window.document.title = `${count} clicks`;
  }, [count]);
  return <button onClick={() => setCount((n) => (n + 1) | 0)}>{count}</button>;
}

export function Ticker() {
  const [ticks, setTicks] = useState(0);
  useEffect(() => {
    const id = setInterval(() => setTicks((n) => (n + 1) | 0), 1000);
    return () => {
      clearInterval(id);
    };
  }, []);
  return <span>{ticks}</span>;
}

export function Logger() {
  useEffect(() => {
    console.log("rendered");
  });
  return <span />;
}
```

- An effect that cleans up only sometimes returns an `Option` of its
  cleanup: `None` is `undefined`, no cleanup.
- `use_layout_effect` and `use_insertion_effect` take the same arguments.
- `use_effect_event(f)` (React 19.2) is a function an effect calls that
  always sees the latest props and state, without being a dependency.

## `use_ref`

`use_ref(initial)` is a box that keeps its value across renders. Read it
with `.current()`, set it with `.set_current(value)`; neither renders
again. For a DOM element, start it at `None` and give it to a tag's `ref`:

```rust
use react::event::MouseEvent;
use react::webapi::HTMLInputElement;
use react::{JSX, jsx, use_ref};

pub fn Search() -> JSX::Element {
    let input = use_ref::<Option<&'static HTMLInputElement>>(None);
    let focus = move |_: &MouseEvent<_>| {
        if let Some(element) = input.current() {
            element.focus(None);
        }
    };
    jsx! {
        <>
            <input ref={input} />
            <button onClick={focus}>{"Focus"}</button>
        </>
    }
}
```

```jsx
import { useRef } from "react";

export function Search() {
  const input = useRef(undefined);
  const focus = () => {
    const element = input.current;
    if (element) {
      element.focus();
    }
  };
  return (
    <>
      <input ref={input} />
      <button onClick={focus}>Focus</button>
    </>
  );
}
```

The ref's element is the tag's or one it extends; another is rustc's
error. `use_imperative_handle(r#ref, create, deps)` sets what a parent's
ref to this component gets: see [refs](api.md#refs).

## `use_memo` and `use_callback`

Each takes its dependencies directly, a tuple, `(todos, tab)`, or `()`:

```rust
use react::{JSX, jsx, use_callback, use_memo, use_state};

pub struct Todo {
    pub text: String,
    pub done: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Tab {
    All,
    Active,
}

fn filter_todos(todos: &[Todo], tab: Tab) -> Vec<&Todo> {
    todos
        .iter()
        .filter(|todo| tab == Tab::All || !todo.done)
        .collect()
}

pub struct TodoListProps {
    pub todos: &'static [Todo],
    pub tab: Tab,
}

pub fn TodoList(TodoListProps { todos, tab }: TodoListProps) -> JSX::Element {
    let visible = use_memo(move || filter_todos(todos, tab), (todos, tab));
    let (picked, set_picked) = use_state(|| 0);
    let pick = use_callback(move |n: usize| set_picked.set(n), ());
    jsx! {
        <p onClick={move |_| pick(visible.len())}>{visible.len()}{" shown, "}{picked}{" picked"}</p>
    }
}
```

```jsx
import { useCallback, useMemo, useState } from "react";

function filterTodos(todos, tab) {
  return todos.filter((todo) => tab === "All" || !todo.done);
}

export function TodoList({ todos, tab }) {
  const visible = useMemo(() => filterTodos(todos, tab), [todos, tab]);
  const [picked, setPicked] = useState(0);
  const pick = useCallback((n) => setPicked(n), []);
  return (
    <p onClick={() => pick(visible.length)}>
      {visible.length} shown, {picked} picked
    </p>
  );
}
```

A dependency tuple has up to twelve values.

## `use_context`

`use_context(&THEME)` reads a context a `thread_local!` holds: the nearest
provider's value, or the context's default. See [context](api.md#context).

## `use_reducer`

`use_reducer(reducer, initial)` gives the state and a `Dispatch`. The
reducer is a function of the state and an action, often an enum:

```rust
use react::{JSX, jsx, use_reducer};

pub enum Action {
    Increment,
    Decrement,
    Set(i32),
}

fn reduce(state: &i32, action: Action) -> i32 {
    match action {
        Action::Increment => state + 1,
        Action::Decrement => state - 1,
        Action::Set(n) => n,
    }
}

pub fn Stepper() -> JSX::Element {
    let (value, dispatch) = use_reducer(reduce, 0);
    jsx! {
        <>
            <button onClick={move |_| dispatch.dispatch(Action::Decrement)}>{"-"}</button>
            {value}
            <button onClick={move |_| dispatch.dispatch(Action::Increment)}>{"+"}</button>
            <button onClick={move |_| dispatch.dispatch(Action::Set(0))}>{"0"}</button>
        </>
    }
}
```

```jsx
import { useReducer } from "react";

function reduce(state, action) {
  if (action === "Increment") {
    return (state + 1) | 0;
  }
  if (action === "Decrement") {
    return (state - 1) | 0;
  }
  return action._0;
}

export function Stepper() {
  const [value, dispatch] = useReducer(reduce, 0);
  return (
    <>
      <button onClick={() => dispatch("Decrement")}>-</button>
      {value}
      <button onClick={() => dispatch("Increment")}>+</button>
      <button onClick={() => dispatch({ TAG: "Set", _0: 0 })}>0</button>
    </>
  );
}
```

An enum's JS shape is [its own page's](../interop/objects.md).
`use_reducer_with(reducer, arg, init)` is `useReducer(reducer, arg, init)`:
the first state is `init(arg)`.

## Custom hooks

A custom hook is a function named `use_..` that calls hooks. It's camelCase
in JS, `useToggle`, the name React's rules and React Compiler look for
([ADR 0573](../../decisions/0573-every-rust-name-is-camel-case-in-js.md)):

```rust
use react::{JSX, jsx, use_state};

pub fn use_toggle(initial: bool) -> (bool, Box<dyn Fn()>) {
    let (on, set_on) = use_state(move || initial);
    (*on, Box::new(move || set_on.update(|on| !on)))
}

pub fn Lamp() -> JSX::Element {
    let (on, toggle) = use_toggle(false);
    jsx! { <button onClick={move |_| toggle()}>{if on { "On" } else { "Off" }}</button> }
}
```

```jsx
import { useState } from "react";

export function useToggle(initial) {
  const [on, setOn] = useState(initial);
  return [on, () => setOn((on) => !on)];
}

export function Lamp() {
  const [on, toggle] = useToggle(false);
  return <button onClick={toggle}>{on ? "On" : "Off"}</button>;
}
```

## Transitions and deferred values

- `use_transition()` gives whether a Transition is pending and what starts
  one: `start.start(move || set_tab.set("posts"))` is
  `start(() => { setTab("posts"); })`. From React 19 the action can be
  `async`.
- `use_deferred_value(value, None)` is `useDeferredValue(value)`; on React
  19, `Some(initial)` is its initial value. On React 18 it takes only
  `value`.
- `start_transition(action)`, outside a component, is React's
  `startTransition`.

## Forms and actions

React 19's form hooks:

```rust
use react::webapi::{FormData, FormDataEntryValue};
use react::{JSX, jsx, use_action_state, use_optimistic};

fn add_name(names: &String, data: &'static FormData) -> String {
    match data.get("name") {
        Some(FormDataEntryValue::Str(name)) => format!("{names}{name} "),
        _ => names.clone(),
    }
}

pub fn Signup() -> JSX::Element {
    let (names, action, pending) = use_action_state(add_name, String::new());
    let (shown, _set_shown) = use_optimistic(names.clone());
    jsx! {
        <form action={action}>
            <input name="name" />
            <button type="submit" disabled={pending}>{"Add"}</button>
            <p>{shown.clone()}</p>
        </form>
    }
}
```

```jsx
import { useActionState, useOptimistic } from "react";

function addName(names, data) {
  const name = data.get("name");
  if (typeof name === "string") {
    return `${names}${name} `;
  }
  return names;
}

export function Signup() {
  const [names, action, pending] = useActionState(addName, "");
  const [shown, _setShown] = useOptimistic(names);
  return (
    <form action={action}>
      <input name="name" />
      <button type="submit" disabled={pending}>
        Add
      </button>
      <p>{shown}</p>
    </form>
  );
}
```

- `use_action_state(action, initial)`: `action` is a function of the
  previous state and the form's `&'static FormData`. Its result is the
  state, the action to give a form's `action`, and whether it's pending.
  `use_async_action_state` takes an `async` action.
- `use_optimistic(value)` and `use_optimistic_with(value, reducer)`: the
  state shown while an action is pending.
- `react::dom::use_form_status()` gives a `FormStatus`, `Pending { data,
  method, .. }` or `NotPending`, of the form a component is in.

## The rest

| Rust | React |
|---|---|
| `use_id()` | `useId()`: a `String` for `id` and `htmlFor` |
| `use_sync_external_store(subscribe, snapshot, server_snapshot)` | `useSyncExternalStore`: `subscribe` gets a `Notify` to `.call()` and returns its unsubscribe |
| `use_debug_value(value, format)` | `useDebugValue` |
| `r#use(&promise)`, `r#use(&THEME)` | React 19's `use`: a promise's value, suspending until it resolves, or a context's. `use` is a Rust keyword, hence `r#use` |
| `use_imperative_handle(r#ref, create, deps)` | `useImperativeHandle` |

## React versions

The crate is built for the React your project has installed, from 18.0 on.
A hook that React added later, `use_effect_event` (19.2), `use_action_state`
(19.0), is a compile error on an older React, which says the version it
needs ([ADR 0043](../../decisions/0043-react-versions.md)):

```text
error[E0432]: unresolved import `react::use_effect_event`
note: found an item that was configured out
    | #[cfg(react = "19.2")]
    |       -------------- the item is gated behind the `19.2` feature
```
