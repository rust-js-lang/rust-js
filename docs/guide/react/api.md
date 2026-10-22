# Context, refs, memo and the rest of React's API

The `react` crate binds all of React's and React DOM's API, as of React
19.3, for whichever React from 18.0 on your project has installed
([`react/coverage.txt`](../../../react/coverage.txt) lists each export).
Each binding is the JS function itself: nothing of the crate is in the JS
it writes.

## Made once: `thread_local!`

JS makes a context, a `memo` component or a `lazy` one once, at its
module's top level. In Rust that's a `thread_local!`, which rust-js
compiles to just that, a module variable ([ADR 0037](../../decisions/0037-thread-locals.md)). Its name is Rust's
`SCREAMING_SNAKE`, and JSX's in JS: a component's or a context's is
capitalized, `THEME` as `Theme`, `FAST_ROW` as `FastRow`, since React
marks those types as components ([ADR 0579](../../decisions/0579-a-thread-local-is-named-as-a-variable.md)). You write no attribute for it.

## Context

```rust
// theme.rs
use react::{Context, create_context};

thread_local! {
    pub static THEME: Context<&'static str> = create_context("light");
}
```

```rust
// lib.rs
mod theme;

use react::{JSX, jsx, use_context};

use theme::THEME;

pub fn Toolbar() -> JSX::Element {
    let theme = use_context(&THEME);
    jsx! { <nav className={*theme}>{"Tools"}</nav> }
}

pub fn App() -> JSX::Element {
    jsx! {
        <THEME value="dark">
            <Toolbar />
        </THEME>
    }
}
```

```jsx
// theme.js
import { createContext } from "react";

export const Theme = createContext("light");
```

```jsx
// lib.jsx
import { useContext } from "react";
import { Theme } from "./theme.js";

export function Toolbar() {
  const theme = useContext(Theme);
  return <nav className={theme}>Tools</nav>;
}

export function App() {
  return (
    <Theme value="dark">
      <Toolbar />
    </Theme>
  );
}
```

- **The provider** is `<THEME value={..}>` on React 19, and
  `<THEME.Provider value={..}>` on React 18 and later. Its children are any
  node.
- **Reading it** is `use_context(&THEME)`, or on React 19 `r#use(&THEME)`,
  which may be called in a condition.
- **`<THEME.Consumer>`** takes a function child of the value, typed:
  `{|user: &&str| jsx! { <p>{*user}</p> }}` is `{(user) => <p>{user}</p>}`.
- **Put a context in a module of its own**, as React advises. Saving a
  file runs its module again, and a context made there is a new one, which
  remounts what's under its provider. rust-js leaves an unchanged
  `theme.js` alone when another file changes, so Fast Refresh keeps the
  state.

## `memo`

```rust
use react::{JSX, MemoExoticComponent, jsx, memo};

pub struct RowProps<'a> {
    pub id: u32,
    pub label: &'a str,
}

pub fn Row(RowProps { id, label }: RowProps) -> JSX::Element {
    jsx! { <li data-id={id}>{label}</li> }
}

thread_local! {
    static FAST_ROW: MemoExoticComponent<RowProps<'static>> = memo(Row, None);
    static ROW_BY_ID: MemoExoticComponent<RowProps<'static>> =
        memo(Row, Some(Box::new(|a, b| a.id == b.id)));
}

pub fn Rows() -> JSX::Element {
    jsx! {
        <ul>
            <FAST_ROW id={1} label="one" />
            <ROW_BY_ID id={2} label="two" />
        </ul>
    }
}
```

```jsx
export function Row({ id, label }) {
  return <li data-id={id}>{label}</li>;
}

const FastRow = memo(Row);
const RowById = memo(Row, (a, b) => a.id === b.id);

export function Rows() {
  return (
    <ul>
      <FastRow id={1} label="one" />
      <RowById id={2} label="two" />
    </ul>
  );
}
```

`memo(component, None)` is `memo(component)`; `Some(are_equal)` is
`memo(component, arePropsEqual)`. The props type names `'static` for its
lifetimes, and the component takes props of any lifetime ([ADR 0265](../../decisions/0265-held-component-lifetimes.md)).

## Refs

### A ref to a DOM element

`use_ref(None)` given to a tag's `ref`: see [hooks](hooks.md#use_ref).

### A component's `ref` prop

As in React 19, a component takes `ref` as a prop: a props field named
`r#ref`. Give it on to an element, or to `use_imperative_handle` to choose
what the parent's ref gets:

```rust
use react::event::MouseEvent;
use react::webapi::{HTMLInputElement, HTMLVideoElement};
use react::{JSX, RefObject, jsx, use_imperative_handle, use_ref};

pub struct FieldProps<'a> {
    pub label: &'a str,
    pub r#ref: RefObject<Option<&'static HTMLInputElement>>,
}

pub fn Field(FieldProps { label, r#ref }: FieldProps) -> JSX::Element {
    jsx! {
        <label>
            {label}
            <input ref={r#ref} />
        </label>
    }
}

pub struct PlayerHandle {
    pub play: Box<dyn Fn()>,
}

pub struct PlayerProps<'a> {
    pub src: &'a str,
    pub r#ref: RefObject<Option<PlayerHandle>>,
}

pub fn Player(PlayerProps { src, r#ref }: PlayerProps) -> JSX::Element {
    let video = use_ref::<Option<&'static HTMLVideoElement>>(None);
    use_imperative_handle(
        r#ref,
        move || PlayerHandle {
            play: Box::new(move || {
                if let Some(element) = video.current() {
                    let _ = element.play();
                }
            }),
        },
        (),
    );
    jsx! { <video src={src} ref={video} /> }
}

pub fn Screen() -> JSX::Element {
    let player = use_ref::<Option<PlayerHandle>>(None);
    let play = move |_: &MouseEvent<_>| {
        if let Some(handle) = player.current() {
            (handle.play)();
        }
    };
    jsx! {
        <>
            <Player src="/intro.mp4" ref={player} />
            <button onClick={play}>{"Play"}</button>
        </>
    }
}
```

```jsx
export function Field({ label, ref }) {
  return (
    <label>
      {label}
      <input ref={ref} />
    </label>
  );
}

export function Player({ src, ref }) {
  const video = useRef(undefined);
  useImperativeHandle(
    ref,
    () => ({
      play: () => {
        const element = video.current;
        if (element) {
          element.play();
        }
      },
    }),
    [],
  );
  return <video src={src} ref={video} />;
}

export function Screen() {
  const player = useRef(undefined);
  const play = () => {
    const handle = player.current;
    if (handle) {
      handle.play();
    }
  };
  return (
    <>
      <Player src="/intro.mp4" ref={player} />
      <button onClick={play}>Play</button>
    </>
  );
}
```

A prop that takes a ref object or a callback is `react::Ref<T>`,
@types/react's `Ref<T>`, given as `ref={Some(object.into())}`. An element's
attributes with its ref are `DetailedHTMLProps<E, T>`, which props
flatten. `create_ref()` is `createRef()`, a new ref each call, as a list
keeps one for each item.

### `forward_ref`

React 18's way, and still React's: `forward_ref(render)` in a
`thread_local!`, its `render` given the props and the parent's ref. Its
tag takes `ref={..}` beside its props, which rustc checks against the
handle type:

```rust
use react::webapi::Element;
use react::{ForwardRefExoticComponent, ForwardedRef, JSX, forward_ref, jsx, use_ref};

pub struct SearchProps<'a> {
    pub placeholder: &'a str,
}

thread_local! {
    static SEARCH_INPUT: ForwardRefExoticComponent<SearchProps<'static>, &'static Element> =
        forward_ref(|SearchProps { placeholder }, input: ForwardedRef<&'static Element>| {
            jsx! { <input placeholder={placeholder} ref={input} /> }
        });
}

pub fn Header() -> JSX::Element {
    let input = use_ref(None);
    jsx! { <SEARCH_INPUT placeholder="Search" ref={input} /> }
}
```

```jsx
const SearchInput = forwardRef(({ placeholder }, input) => (
  <input placeholder={placeholder} ref={input} />
));

export function Header() {
  const input = useRef(undefined);
  return <SearchInput placeholder="Search" ref={input} />;
}
```

## `lazy` and `Suspense`

`lazy` in a `thread_local!` loads a module whose default export is a
component, the first time it renders. `import_module` names the JS file as
the bundler sees it. Render it inside `<Suspense>`:

```rust
use react::{JSX, LazyExoticComponent, import_module, jsx, lazy};

thread_local! {
    static CHART: LazyExoticComponent<()> = lazy(|| import_module("./chart.jsx"));
}

pub fn Dashboard() -> JSX::Element {
    jsx! {
        <Suspense fallback={jsx! { <p>{"Loading chart"}</p> }}>
            <CHART />
        </Suspense>
    }
}
```

```jsx
const Chart = lazy(() => import("./chart.jsx"));

export function Dashboard() {
  return (
    <Suspense fallback={<p>Loading chart</p>}>
      <Chart />
    </Suspense>
  );
}
```

A Rust module's default export is `js::export_default!(Chart);`
([modules](../project/modules.md)). For a module whose component isn't its
default, `Module::of(component)` is `{ default: component }`. `<Suspense>`
takes `fallback` and `name`.

## Built-in components

Each is a tag `jsx!` knows, imported from `react` where it's used:

| Tag | Props |
|---|---|
| `<Fragment key={..}>` | `key`; on 19.3, a `ref` to a `FragmentInstance` |
| `<StrictMode>` | none |
| `<Suspense>` | `fallback`, `name` |
| `<Profiler>` | `id`, `onRender={\|id, phase, actual, base, start, commit\| ..}` |
| `<Activity>` (19.2) | `mode={ActivityMode::Hidden}` or `Visible`, `name` |
| `<ViewTransition>` (19.3) | `name`, `enter`, `exit`, `update`, `share`, `default`, `onEnter` and the other events, `ref` |

```rust
use react::{JSX, Phase, jsx};

pub fn App() -> JSX::Element {
    jsx! {
        <Profiler
            id="app"
            onRender={|id, phase, actual, _, _, _| {
                if matches!(phase, Phase::Mount) {
                    println!("{id} mounted in {actual}ms");
                }
            }}
        >
            <h1>{"Hello"}</h1>
        </Profiler>
    }
}
```

```jsx
export function App() {
  return (
    <Profiler
      id="app"
      onRender={(id, phase, actual) => {
        if (phase === "mount") {
          console.log(`${id} mounted in ${$displayF64(actual)}ms`);
        }
      }}
    >
      <h1>Hello</h1>
    </Profiler>
  );
}
```

`$displayF64` writes the number as Rust's `{}` does, which isn't always
JS's (see [semantics](../semantics.md)).

## Portals, roots and server rendering

React DOM is `react::dom`:

```rust
use react::dom::client::{RootOptions, create_root};
use react::dom::create_portal;
use react::dom::server::render_to_string;
use react::webapi::{Element, window};
use react::{JSX, ReactNode, jsx};

pub struct ModalProps<'a> {
    pub children: &'a dyn ReactNode,
}

pub fn Modal(ModalProps { children }: ModalProps) -> Option<JSX::Element> {
    let body = window.document().body()?;
    let content = jsx! { <div className="modal">{children}</div> };
    Some(create_portal(content, body, None).element())
}

pub fn mount(container: &Element) {
    let root = create_root(container, Some(RootOptions::new().identifier_prefix("app-")));
    root.render(jsx! {
        <StrictMode>
            <App />
        </StrictMode>
    });
}

pub fn html() -> String {
    render_to_string(jsx! { <App /> }, None)
}
```

```jsx
export function Modal({ children }) {
  if (!window.document.body) {
    return;
  }
  const body = window.document.body;
  const content = <div className="modal">{children}</div>;
  return createPortal(content, body);
}

export function mount(container) {
  const root = createRoot(container, { identifierPrefix: "app-" });
  root.render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}

export function html() {
  return renderToString(<App />);
}
```

- **Options are objects built by methods**:
  `RootOptions::new().identifier_prefix("app-")` is `{ identifierPrefix:
  "app-" }`. A trailing `None` is left out.
- **`react::dom`**: `create_portal`, `flush_sync`, the resource hints
  (`prefetch_dns`, `preconnect`, `preload`, `preload_module`, `preinit`,
  `preinit_module`), `use_form_status`, `request_form_reset`.
- **`react::dom::client`**: `create_root` and `hydrate_root`, whose `Root`
  has `render` and `unmount`, and `RootOptions`, with the root's error
  callbacks, `on_caught_error` and the rest. Error boundaries are a class
  component's, so these callbacks are where a render's errors go.
- **`react::dom::server`**: `render_to_string`, `render_to_static_markup`,
  `render_to_readable_stream`, `render_to_pipeable_stream`, `resume` and
  `resume_to_pipeable_stream`.
- **`react::dom::prerender`**: `react-dom/static`'s `prerender`,
  `prerender_to_node_stream` and their `resume_and_` forms.

A portal is a `ReactPortal`: a child as it is, and a component's result by
`.element()`.

## Children

To look inside a component's children, as react.dev's MDX components do:

- `react::children::to_array(&children)` is `Children.toArray(children)`,
  a `Vec<Child>`, each `Child::Text`, `Child::Number`, `Child::Element` or
  `Child::Other`, told apart by `match` as JS tells them by `typeof` and
  `isValidElement`.
- `react::children::for_each(&children, |child, index| ..)` is
  `Children.forEach`; an empty child is `None`.
- `react::kind_of(&node)` tells any node apart the same way, a
  `ReactNodeKind`.
- A `ReactElement` has `.r#type()`, `.props()` and `.key()`;
  `ReactElement::<P>::unchecked_from(e)` reads its props as `P`.
- `clone_element(element, props)` is `cloneElement(element, props)`.

## Other APIs

| Rust | React |
|---|---|
| `start_transition(action)` | `startTransition` |
| `add_transition_type(name)` (19.3) | `addTransitionType` |
| `act(f)` (18.3) | `act`, in tests |
| `cache(f)` (19.0) | `cache`, for Server Components |
| `cache_signal()` (19.2) | `cacheSignal`, `None` outside one |
| `capture_owner_stack()` (19.1) | `captureOwnerStack` |
| `react::VERSION` | `version` |
| `FRAGMENT`, `element_type(tag)` | `Fragment` as a value, an `ElementType` |

## What's not bound

- **Class components**, `Component` and `PureComponent`, and so error
  boundaries: a Rust program has no classes.
- **`createElement`**: elements are made by [`jsx!`](jsx.md), the one way.
- React DOM's legacy `unstable_batchedUpdates` and `useFormState`, which
  React 19 renamed `useActionState`.
- TypeScript's own types, as `coverage.txt` lists them.

## React versions

What a React release after 18.0 added is gated by it, `#[cfg(react =
"19.2")]`, and the crate is built for the React your project has
installed: using what it lacks is a compile error that names the release
([ADR 0043](../../decisions/0043-react-versions.md)). [`react/versions.json`](../../../react/versions.json) records
which release first has each export, event and attribute.
