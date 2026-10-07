# 0227. The react crate's types have @types/react's names

Status: Accepted. Renames what [0041](0041-react.md), [0043](0043-react-versions.md),
[0208](0208-react-attributes.md), [0224](0224-typed-intrinsic-elements.md) and
[0226](0226-react-children.md) named.

## Context

A component's children in TypeScript are `children: React.ReactNode`; in
the react crate they were `C: Node`. A style was a `Style`, where React's is
`CSSProperties`; `useRef`'s object a `Ref`, where React's is `RefObject`;
a click's event `event::Mouse`, where React's is `MouseEvent`. Someone who
knows React, or ports its TypeScript, had to learn a second set of names
for the same things, and the `.d.ts` rust-js writes already used React's.

## Decision

**A type of the react crate that stands for one of @types/react's, or
@types/react-dom's, has its name, spelled as it is there.**

| Was | Is |
|---|---|
| trait `Node`, `NodeKind` | `ReactNode`, `ReactNodeKind` |
| `Style` | `CSSProperties` |
| `Ref<T>` | `RefObject<T>` |
| trait `RefValue<H, M>`, a JSX ref | `Ref<H, M>`, with `RefCallback<T, C = ()>` (Amended) |
| `SetState<T>` | `Dispatch<SetStateAction<T>>` |
| `StartTransition` | `TransitionStartFunction` |
| trait `Deps` | `DependencyList` |
| trait `Component<P, M>` | `ComponentType<P, M>` |
| `Memo<P>`, `Lazy<P>`, `ForwardRef<P, H>` | `MemoExoticComponent<P>`, `LazyExoticComponent<P>`, `ForwardRefExoticComponent<P, H>` |
| `Provider<T, C>` (its props), `ContextProvider<T>` | `ProviderProps<T, C>`, `Provider<T>` |
| `event::Event`, `event::Mouse`, `Ui`, `Change`, … | `event::SyntheticEvent`, `event::MouseEvent`, `UIEvent`, `ChangeEvent`, … |
| `AnchorHtmlAttributes`, `HtmlAttributes`, `SvgAttributes`, … | `AnchorHTMLAttributes`, `HTMLAttributes`, `SVGAttributes`, … |
| `dom::server::RenderStream` | `ReactDOMServerReadableStream` |
| `Postponed`, `Prerendered` | `PostponedState`, `PrerenderResult` |

- **`useState`'s setter is `Dispatch<SetStateAction<T>>`**, as React types
  it: `SetStateAction<T>` is what it takes, a value, `set`, or a function of
  the previous one, `update`; the reducer's `Dispatch<A>` is the same type
  of another action.
- **A handler is its alias**, as @types/react's: `event::EventHandler<E>`
  is `Box<dyn Fn(&E)>`, and `MouseEventHandler<T = Element>` an
  `EventHandler<MouseEvent<T>>`, one per event, `ReactEventHandler`
  `SyntheticEvent`'s; the generated attributes use them, `on_click:
  Option<event::MouseEventHandler>`. `ChangeEventHandler<T>` takes one
  element, as this `ChangeEvent` does. Each says it's @types/react's,
  `#[rust_js::types = "react#MouseEventHandler<T>"]`, so a `.d.ts` writes
  it as it's written, `onClick?: MouseEventHandler<HTMLButtonElement>` (ADR
  0196), as `RefCallback`, `FormEvent`, `Reducer` and `EffectCallback` too.
  (Amended: the `.d.ts` wrote
  the alias expanded, `(event: MouseEvent<HTMLButtonElement>) => void`.)
- **An event's native event is its type**, as @types/react's: a
  `SyntheticEvent<T, E = Event>`'s `native_event()` is an `&E`, and a
  `MouseEvent<T, E = MouseEvent>` a `UIEvent<T, E>`, where a `PointerEvent<T>`
  is a `MouseEvent<T, PointerEvent>`, so a click's `native_event()` is a
  `webapi::MouseEvent` and a pointer's a `webapi::PointerEvent`, where each
  was a `webapi::Event`. Only `SyntheticEvent`, `UIEvent` and `MouseEvent`
  take `E`, as React's. `#[rust_js::types]` names `T` alone, so the `.d.ts`
  writes `MouseEvent<HTMLButtonElement>`, its `E` the default. webapi binds
  the native events React's wrap (ADR 0024). (Amended.)
- **A form's submit is a `SubmitEvent`**, as @types/react's `onSubmit` is,
  whose `submitter()` is the button that sent it and whose native event is
  webapi's `SubmitEvent`, where it was a `SyntheticEvent`; with
  `SubmitEventHandler`. `FormEvent` and `InvalidEvent`, which add nothing
  to a `SyntheticEvent` in @types/react, are its names, with
  `FormEventHandler`. (Amended.)
- **`onInput` is an `InputEvent`**, as @types/react's `InputEventHandler`
  types it, of `data()` and a native `InputEvent`, as `onBeforeInput` is,
  where it was a `ChangeEvent`, of `value()`: @types/react's types are the
  rule, over a convenience of the crate's own. Its value is its element's,
  `html_input_element::value(e.current_target())`. (Amended.)
- **A ref is @types/react's `Ref`**, the trait of what JSX's `ref` takes, a
  `RefObject` or any `Fn(Option<H>)`, as `Ref<T>` is TypeScript's union of
  them (ADR 0229), its error saying so, `` `RefObject<Option<i32>>` is not a
  `Ref` of `&Element` ``; and `RefCallback<T, C = ()>`, a callback a prop
  holds, `Box<dyn Fn(Option<T>) -> C>`, its `C` the cleanup React 19 runs.
  (Amended.)
- **A context has its `Consumer`**, `<THEME.Consumer>{|theme| ..}</THEME.Consumer>`,
  as `<THEME.Provider>` is its member, of `ConsumerProps`, whose children
  are a function of its value, as `use_context` gives it. **A portal is a
  `ReactPortal`**, `create_portal`'s, where it was an `Element`: an
  element, `Deref` to a `ReactElement`, as `ReactPortal extends
  ReactElement`, a child as it is, a component's result by `.element()`.
  (Amended.)
- **What a hook takes and gives has @types/react's names**: `Reducer<S, A>`,
  `Box<dyn Fn(&S, A) -> S>`, `use_reducer`'s; `ActionDispatch<A>`, the
  `Dispatch<A>` it gives; `EffectCallback<C = ()>`, `use_effect`'s; and
  `TransitionFunction<R = ()>`, `start_transition`'s, each of a prop given to
  its hook as it is. Not `ReducerWithoutAction` nor `DispatchWithoutAction`:
  `use_reducer` takes an action, and `()` given to JS is `[]`. (Amended.)
- **Names React has no type for stay the crate's own**: `Element`, what
  JSX makes, `JSX.Element`; `Rest`, `...props`; `children::Child`;
  `InnerHtml`, `{ __html }`; `Cleanup`, what an effect gives back, which
  React calls a `Destructor`, a word Rust already has for `Drop`.
- **Generators keep them**: `react/generate.ts` and `react/attributes.ts`
  write the events' and the attributes' names as @types/react has them.
- **webapi's types have the DOM's names too**, as WebIDL and TypeScript's
  DOM lib spell them: `HTMLButtonElement`, `DOMRect`, `UIEvent`, where
  they were web-sys's, `HtmlButtonElement` (ADR 0024). (Amended.)

## Why

- **One vocabulary**: React's docs, its TypeScript, the `.d.ts` rust-js
  writes, and the Rust all name a thing the same.
- **It's checked**: the react crate, its tests, the examples, the
  playground and the react.dev port build with the names, and the JS they
  make is the same, as names are Rust's only.

## Costs

- **A breaking change** to every crate that names one of these types; the
  JS doesn't change.
