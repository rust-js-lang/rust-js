# Build UIs with React

You'll build a small React app in Rust: a counter with props, a badge with
children, and a todo list with a reducer, a form and an effect. Each step
shows the Rust you write and the JSX rust-js writes from it.

Start from the app [Getting started](vite.md) made. Run `bun run dev` in
one terminal, and `bunx rust-js build` in another after each edit.

Every example here was compiled by rust-js as it is today; the JS shown is
what it writes, without its header comment and source-map line.

## The rules

- **A component is a `pub fn` with a capitalized name that returns
  `JSX::Element`**, as in React. It takes no parameter, or one: its props
  struct.
- **`jsx!` is JSX.** Tags and attributes are React's; what's in braces is
  Rust. Text is a Rust string, `{"Hello"}`. Import it as any macro:
  `use react::jsx;`.
- **Attributes are spelled as React spells them**: `className`, `onClick`,
  `aria-label`. A props field `rich_colors` is the prop `richColors`, and
  is written so in `jsx!` ([ADR 0573](../../decisions/0573-every-rust-name-is-camel-case-in-js.md)).
- **Hooks are React's, in snake case**: `use_state`, `use_reducer`,
  `use_effect`, `use_ref`, `use_memo`.

[JSX with `jsx!`](../react/jsx.md) has the full syntax, and
[Hooks](../react/hooks.md) every hook.

## A component with props

Props are a struct. Destructure it in the parameter, as you would in JS.
Write the counter in its own module, `src/counter.rs`, and use it from
`App` with its props as attributes:

```rust
// src/App.rs
mod counter;

use counter::Counter;
use react::{JSX, jsx};

pub fn App() -> JSX::Element {
    jsx! {
        <main>
            <Counter label="Apples" step={1} />
            <Counter label="Boxes of apples" step={12} />
        </main>
    }
}
```

```rust
// src/counter.rs
use react::{JSX, jsx, use_state};

pub struct CounterProps {
    pub label: &'static str,
    pub step: i32,
}

/// A count of `label`, that goes up by `step`.
pub fn Counter(CounterProps { label, step }: CounterProps) -> JSX::Element {
    let (count, set_count) = use_state(|| 0);

    jsx! {
        <p>
            {label}{": "}{count}{" "}
            <button type="button" onClick={move |_| set_count.update(move |count| count + step)}>
                {"Add "}{step}
            </button>
            <button type="button" disabled={*count == 0} onClick={move |_| set_count.set(0)}>
                {"Reset"}
            </button>
        </p>
    }
}
```

```jsx
// src/App.jsx
import { Counter } from "./counter.jsx";

export function App() {
  return (
    <main>
      <Counter label="Apples" step={1} />
      <Counter label="Boxes of apples" step={12} />
    </main>
  );
}
```

```jsx
// src/counter.jsx
import { useState } from "react";

/** A count of `label`, that goes up by `step`. */
export function Counter({ label, step }) {
  const [count, setCount] = useState(0);

  return (
    <p>
      {label}: {count}{" "}
      <button type="button" onClick={() => setCount((count) => (count + step) | 0)}>
        Add {step}
      </button>
      <button type="button" disabled={count === 0} onClick={() => setCount(0)}>
        Reset
      </button>
    </p>
  );
}
```

- **rustc checks every prop.** A missing prop, an unknown one, or one of the
  wrong type is a compile error.
- **State is `use_state`.** It gives back the value and its setter.
  `set.set(value)` is `setCount(value)`; `set.update(|old| new)` is
  `setCount((old) => new)`.
- **What a hook gives back is `Copy`**, so each handler can `move` it in.
  It's a `&'static T`, read-only as React's state is: `*count == 0`
  compares the value.
- **A handler is a closure**, `move |_| ..`, and an arrow function in JS.
- **`counter.d.ts` declares the props** for TypeScript callers:
  `export interface CounterProps { label: string; step: number; }`.

## Children and optional props

A field named `children` holds what's between the tags. A field that's an
`Option` is a prop the caller may leave out: give it `Some(value)`, or
leave it out for `None` ([ADR 0438](../../decisions/0438-an-optional-prop-is-given-some.md)).

```rust
use react::{JSX, jsx};

pub struct BadgeProps {
    pub text: &'static str,
    pub color: Option<&'static str>,
    pub children: JSX::Element,
}

pub fn Badge(props: BadgeProps) -> JSX::Element {
    let class_name = props.color.unwrap_or("gray");
    jsx! {
        <span className={class_name}>{props.text}{props.children}</span>
    }
}

pub fn Badges() -> JSX::Element {
    jsx! {
        <>
            <Badge text="New"><b>{"!"}</b></Badge>
            <Badge text="Sale" color={Some("red")}><i>{"%"}</i></Badge>
        </>
    }
}
```

```jsx
export function Badge(props) {
  const className = props.color ?? "gray";
  return (
    <span className={className}>
      {props.text}
      {props.children}
    </span>
  );
}

export function Badges() {
  return (
    <>
      <Badge text="New">
        <b>!</b>
      </Badge>
      <Badge text="Sale" color="red">
        <i>%</i>
      </Badge>
    </>
  );
}
```

- **`<>...</>` is a fragment**, as in JSX.
- **`None` is `undefined`.** `unwrap_or` is `??`, and the `.d.ts` declares
  the prop `color?: string | undefined`.

See [Components and props](../react/components.md) for the rest: spreads,
generic components, `ref`.

## A todo list

The todo list holds its todos in a reducer, its draft in state, and sets
the page's title in an effect.

```rust
// src/todos.rs
use react::event::{ChangeEvent, SubmitEvent};
use react::{Dispatch, JSX, jsx, use_effect, use_reducer, use_state};
use webapi::document;

#[derive(Clone)]
struct Todo {
    id: u32,
    title: String,
    done: bool,
}

#[derive(Clone, Default)]
struct Todos {
    items: Vec<Todo>,
    next_id: u32,
}

enum Action {
    Add(String),
    Toggle(u32),
    Remove(u32),
}

fn reduce(todos: &Todos, action: Action) -> Todos {
    let mut next = todos.clone();
    match action {
        Action::Add(title) => {
            next.items.push(Todo {
                id: next.next_id,
                title,
                done: false,
            });
            next.next_id += 1;
        }
        Action::Toggle(id) => {
            if let Some(todo) = next.items.iter_mut().find(|todo| todo.id == id) {
                todo.done = !todo.done;
            }
        }
        Action::Remove(id) => next.items.retain(|todo| todo.id != id),
    }
    next
}

struct TodoItemProps {
    todo: Todo,
    dispatch: Dispatch<Action>,
}

fn TodoItem(TodoItemProps { todo, dispatch }: TodoItemProps) -> JSX::Element {
    let id = todo.id;
    jsx! {
        <li>
            <label>
                <input
                    type="checkbox"
                    checked={todo.done}
                    onChange={move |_: &ChangeEvent<_>| dispatch.dispatch(Action::Toggle(id))} />
                {todo.title}
            </label>
            <button type="button" onClick={move |_| dispatch.dispatch(Action::Remove(id))}>{"Remove"}</button>
        </li>
    }
}

pub fn TodoList() -> JSX::Element {
    let (todos, dispatch) = use_reducer(reduce, Todos::default());
    let (draft, set_draft) = use_state(|| String::new());
    let left = todos.items.iter().filter(|todo| !todo.done).count();

    use_effect(
        move || document.set_title(&format!("{left} left")),
        Some(&(left,)),
    );

    let add = move |e: &SubmitEvent<_>| {
        e.prevent_default();
        let title = draft.trim();
        if !title.is_empty() {
            dispatch.dispatch(Action::Add(title.to_string()));
            set_draft.set(String::new());
        }
    };

    jsx! {
        <section>
            <form onSubmit={add}>
                <input
                    placeholder="What needs to be done?"
                    value={draft.clone()}
                    onChange={move |e: &ChangeEvent<_>| set_draft.set(e.value())} />
                <button type="submit">{"Add"}</button>
            </form>
            <ul>
                {todos
                    .items
                    .iter()
                    .map(|todo| jsx! { <TodoItem key={todo.id} todo={todo.clone()} dispatch={dispatch} /> })
                    .collect::<Vec<_>>()}
            </ul>
            <p>{left}{if left == 1 { " item left" } else { " items left" }}</p>
        </section>
    }
}
```

```jsx
// src/todos.jsx
import { useEffect, useReducer, useState } from "react";
import { $retain, $trim } from "@rust-js/runtime";

function reduce(todos, action) {
  const next = { ...todos, items: todos.items.map((item) => ({ ...item })) };
  if (action.TAG === "Add") {
    next.items.push({
      id: next.nextId,
      title: action._0,
      done: false,
    });
    next.nextId = (next.nextId + 1) >>> 0;
  } else if (action.TAG === "Toggle") {
    const captured = action._0;
    const todo = next.items.find((todo) => todo.id === captured);
    if (todo != null) {
      todo.done = !todo.done;
    }
  } else {
    const captured = action._0;
    $retain(next.items, (todo) => todo.id !== captured);
  }
  return next;
}

function TodoItem({ todo, dispatch }) {
  const id = todo.id;
  return (
    <li>
      <label>
        <input
          type="checkbox"
          checked={todo.done}
          onChange={() => dispatch({ TAG: "Toggle", _0: id })}
        />
        {todo.title}
      </label>
      <button type="button" onClick={() => dispatch({ TAG: "Remove", _0: id })}>
        Remove
      </button>
    </li>
  );
}

export function TodoList() {
  const [todos, dispatch] = useReducer(reduce, { items: [], nextId: 0 });
  const [draft, setDraft] = useState("");
  const left = todos.items.filter((todo) => !todo.done).length;

  useEffect(() => {
    document.title = `${left} left`;
  }, [left]);

  function add(e) {
    e.preventDefault();
    const title = $trim(draft);
    if (title) {
      dispatch({ TAG: "Add", _0: title });
      setDraft("");
    }
  }

  return (
    <section>
      <form onSubmit={add}>
        <input
          placeholder="What needs to be done?"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
        />
        <button type="submit">Add</button>
      </form>
      <ul>
        {todos.items.map((todo) => (
          <TodoItem key={todo.id} todo={{ ...todo }} dispatch={dispatch} />
        ))}
      </ul>
      <p>
        {left}
        {left === 1 ? " item left" : " items left"}
      </p>
    </section>
  );
}
```

What each part became:

- **The reducer is a Rust function**, `fn(&S, A) -> S`, given to
  `use_reducer` as it is to `useReducer`. Its `clone()` is a copy of the
  object and of each todo in it, so the old state is never changed, as
  React expects.
- **An enum is a tagged object.** `Action::Toggle(id)` is
  `{ TAG: "Toggle", _0: id }`, and `match` tests its `TAG`. See
  [Structs, enums and traits](../interop/objects.md).
- **`Dispatch` is `Copy`**, so the reducer's `dispatch` is passed to each
  item as a prop, and moved into each handler.
- **A list is a `Vec` of elements**: `.map(..).collect::<Vec<_>>()` is
  `.map(..)`. Give each element a `key`, as React asks; `key` is not one of
  the component's props.
- **A conditional child is an `if`**: an `if` with an `else` is `? :`. An
  `Option` child renders nothing for `None`.
- **An effect's dependencies are a tuple.** `Some(&(left,))` is `[left]`,
  `Some(&())` is `[]`, and `None` leaves them out. Its closure returns
  nothing, or the closure that cleans it up.
- **A handler's event is typed by its tag.** A `<form>`'s `onSubmit` gets a
  `SubmitEvent` of an `HTMLFormElement`. A closure written outside the JSX
  names its event, `&SubmitEvent<_>`, and the tag fills in the element.
  `e.value()` is the input's `e.target.value`.
- **`webapi` is the DOM.** `document.set_title(..)` is
  `document.title = ..`. See [The crates rust-js ships](../libraries/README.md).
- **`@rust-js/runtime` holds what Rust does that JS doesn't**, imported
  where it's used: `$retain` is `Vec::retain`, and `$trim` is `str::trim`,
  which trims what Rust calls whitespace. See
  [Semantics](../semantics.md).

Then show the list in `App`, after the counters:

```rust excerpt
// src/App.rs
mod counter;
mod todos;

use counter::Counter;
use react::{JSX, jsx};
use todos::TodoList;

pub fn App() -> JSX::Element {
    jsx! {
        <main>
            <Counter label="Apples" step={1} />
            <Counter label="Boxes of apples" step={12} />
            <TodoList />
        </main>
    }
}
```

`App.jsx` imports it as it does `Counter`, from `./todos.jsx`.

## Mount it

The template's `src/main.jsx` mounts `App` as any Vite app does, and you
don't change it:

```jsx
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import { App } from './App.jsx'

createRoot(document.getElementById('root')).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
```

rust-js exports each component by name, so it's `import { App }`.

## Next

- [Use a JavaScript library from Rust](js-library.md): an npm component and
  its functions, bound and called.
- [Hooks](../react/hooks.md), [Context, refs, memo and the rest](../react/api.md).
- [Next.js](../react/nextjs.md): the same components, in a Next.js app.
