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
