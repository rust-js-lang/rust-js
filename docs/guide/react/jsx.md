# JSX with `jsx!`

`jsx!` is JSX in Rust: tags and attributes as JSX writes them, and Rust
expressions in braces. rust-js compiles it to the JSX you'd write by hand,
in a `.jsx` module. rustc then checks every tag, prop and handler against
the `react` crate's types.

```rust
use react::{JSX, jsx, use_state};

pub fn Counter() -> JSX::Element {
    let (count, set_count) = use_state(|| 0);
    jsx! {
        <button className="counter" onClick={move |_| set_count.update(|n| n + 1)}>
            {"Count is "}{count}
        </button>
    }
}
```

```jsx
import { useState } from "react";

export function Counter() {
  const [count, setCount] = useState(0);
  return (
    <button className="counter" onClick={() => setCount((n) => (n + 1) | 0)}>
      Count is {count}
    </button>
  );
}
```

- **Import it**, `use react::jsx;`, and call it by that name. A path to it,
  `react::jsx! { .. }`, is an error that says to import it.
- **It's built into rust-js**, in the native compiler and the playground's
  alike, the same parser. No procedural macro or generated Rust file is
  involved.
- **To a plain rustc**, your editor's or `cargo check`'s, `jsx!` is an
  `Element` it doesn't look inside, so the rest of the crate type-checks
  there too ([ADR 0113](../../decisions/0113-plain-rustc.md)). To check inside the markup in your editor, run
  rust-analyzer's check through rust-js, `rust-js-check` as its
  `check.overrideCommand` ([ADR 0222](../../decisions/0222-editor-check.md)).
- **Source maps** point at the Rust, down to each statement of a handler.

## Elements and children

| Syntax | Meaning |
|---|---|
| `<div />`, `<div>..</div>` | An HTML element; `<svg>`, `<circle>` and the rest are SVG's |
| `<Card title="Hi" />` | A component, a capitalized function |
| `<>..</>` | A fragment |
| `<Fragment key={id}>..</Fragment>` | A keyed fragment |
| `{"Hello "}`, `"Hello "` | Text, its whitespace kept as written |
| `{count}` | Any Rust expression |
| `{let n = compute(); n + 1}` | A block with statements |
| `{items}` | A `Vec` of elements, a list; each needs a `key` |
| `{maybe}` | An `Option`: nothing where it's `None` |
| `{/* comment */}` | A comment, which renders nothing |

One `jsx!` makes one element. Several side by side are an error, `wrap
adjacent JSX elements in <>...</>`.

### Text

Text is a Rust string literal, quoted. Bare words are an error, `expected a
literal or a Rust expression in {braces}`, and HTML entities aren't read:
write `{"A & B"}`, which stays `{"A & B"}` in the JS. Adjacent text children
stay separate text, as written.

### Expressions

Braces hold any Rust expression, children's and attributes' alike: a
`match`, an `if`, a method chain, a block. What a child can be is
`ReactNode`: an element, `&str`, `String`, a number, a `bool`, `()`, and an
`Option`, a `Vec`, a tuple or a reference of those. A tuple is several
children; there's no limit on how many siblings a tag has.

```rust
use react::{CSSProperties, JSX, inner_html, jsx};

pub struct Item {
    pub id: u32,
    pub name: String,
}

pub struct ListProps<'a> {
    pub items: &'a [Item],
}

pub fn Basics(ListProps { items }: ListProps) -> JSX::Element {
    let count = items.len();
    jsx! {
        <div className="list" aria-label="Items" data-count={count}>
            {/* a comment renders nothing */}
            <h2>{"Items: "}{count}</h2>
            <input type="checkbox" disabled />
            <p style={CSSProperties::new().color("gray").font_size(12)}>{"A & B"}</p>
            <div dangerouslySetInnerHTML={inner_html("<b>raw</b>")} />
            {let first = items.first().map(|item| item.name.clone()); first}
        </div>
    }
}
```

```jsx
export function Basics({ items }) {
  const count = items.length;
  const first = items[0]?.name;
  return (
    <div className="list" aria-label="Items" data-count={count}>
      <h2>Items: {count}</h2>
      <input type="checkbox" disabled />
      <p style={{ color: "gray", fontSize: 12 }}>{"A & B"}</p>
      <div dangerouslySetInnerHTML={{ __html: "<b>raw</b>" }} />
      {first}
    </div>
  );
}
```

Lists and conditions are Rust's iterators, `then`, `map` and `if`: see
[components](components.md#rendering-lists).

`jsx!` works inside another macro's arguments too:
`vec![jsx! { <li key="a">{"a"}</li> }, jsx! { <li key="b">{"b"}</li> }]` is
`[<li key="a">a</li>, <li key="b">b</li>]`.

## Attributes

An attribute is written by its JSX name, and only so: `className`,
`htmlFor`, `onClick`, `tabIndex`, `aria-label`, `data-count`; an SVG
element's are SVG's, `viewBox`, `strokeWidth`. Rust's spelling is an error
that names JSX's ([ADR 0435](../../decisions/0435-rust-and-react-two-worlds.md)):

```text
error: rust-js: `on_click` is `onClick` in JSX, as React names it
error: rust-js: `aria_label` is `aria-label` in JSX, as React names it
```

- **The value** is a literal, `"text"`, `3`, or a Rust expression in braces.
  An attribute alone, `disabled`, is `true`.
- **Its type is @types/react's** for that tag: `disabled` a `bool`,
  `width` a number or a string, `onChange` a handler of the tag's element.
  A wrong one is rustc's error. An `Option` of a value leaves the attribute
  out where it's `None`.
- **A few strings**, as @types/react types `type` on a `<button>`, are
  checked when written as a literal:
  `` `"sumbit"` isn't a `type`, which is one of "button", "reset", "submit" ``.
- **Any `aria-*` or `data-*`** attribute is taken, its value text, a number
  or a `bool`.
- **`style`** is a `CSSProperties`, built by a method for each CSS
  property, typed as csstype types them: `CSSProperties::new().color("red").font_size(12)`
  is `{{ color: "red", fontSize: 12 }}`. `.set("--gap", "4px")` sets a
  custom property, and `.spread(other)` another style's properties over
  these, `{{ width, ...customStyles }}`. A style kept elsewhere is passed by
  reference, `style={&style}`, or as an `Option` of one.
- **`dangerouslySetInnerHTML`** takes `inner_html(html)`, `{{ __html: html }}`.
- **A duplicate attribute** is an error.

### Handlers

A handler attribute takes a closure of the tag's event, `onClick={move |_|
..}`. The event is of the tag's DOM element ([ADR 0224](../../decisions/0224-typed-intrinsic-elements.md)): a `<button>`'s
`onClick` gets an `event::MouseEvent<webapi::HTMLButtonElement>`, whose
`current_target()` is the button. A closure written outside the JSX names
its event `&event::MouseEvent<_>`, and the tag fills in its element. See
[events](components.md#events).

### Refs

`ref` on a DOM tag takes a ref object, `use_ref(None)`'s, or a callback,
of the tag's element or one it extends. An `<input>`'s ref on a `<button>`
is rustc's error, `is not a ref of`. A callback may return the cleanup
React 19 calls when it's detached:

```rust
use react::webapi::{FormData, HTMLDivElement};
use react::{JSX, jsx};

fn save(data: &'static FormData) {
    if data.has("title") {
        println!("saved");
    }
}

pub fn Editor() -> JSX::Element {
    jsx! {
        <form action={save}>
            <div
                ref={|node: Option<&'static HTMLDivElement>| {
                    if let Some(node) = node {
                        node.scroll_into_view(None);
                    }
                }}
            />
            <input name="title" />
            <button formAction="/draft">{"Save draft"}</button>
        </form>
    }
}
```

```jsx
function save(data) {
  if (data.has("title")) {
    console.log("saved");
  }
}

export function Editor() {
  return (
    <form action={save}>
      <div
        ref={(node) => {
          if (node) {
            node.scrollIntoView();
          }
        }}
      />
      <input name="title" />
      <button formAction="/draft">Save draft</button>
    </form>
  );
}
```

A component's `ref` is a props field named `r#ref`, as in React 19; a
`forward_ref` component takes `ref` beside its props. See
[refs](api.md#refs).

### Form actions

`action` and `formAction` take a URL, or on React 19, a function of the
form's `&'static FormData`, `async` or not, or the action
`use_action_state` gives. See [hooks](hooks.md#forms-and-actions).

## Spreads

On a DOM tag, `{...attrs}` spreads a struct's fields as attributes, by
their JS names. It goes anywhere among the attributes, and what's written
after it wins, as in JSX:

```rust
# use react::attributes::HTMLAttributes;
# use react::{JSX, jsx};
#
pub struct SpreadProps<'a> {
    pub attrs: HTMLAttributes<'a>,
}

pub fn Spread(SpreadProps { attrs }: SpreadProps) -> JSX::Element {
    jsx! { <section id="main" {...attrs} title="Main" /> }
}
```

```jsx
export function Spread({ attrs }) {
  return <section id="main" {...attrs} title="Main" />;
}
```

On a component, there are two forms, each one way to do its job:

- **`{...props}` alone** is the component's props, whole: `<Title
  {...props} />`. Children may follow it, and replace its `children`.
- **Named props and `{..base}`**, Rust's struct update, last, take every
  prop not named from `base`: `<Title text="New" {..props} />`. The base is
  evaluated last, as Rust's is, and what's named wins.

Named props with JSX's `{...base}` are an error, since JSX's spread would
override them where Rust's struct update doesn't:

```text
error: jsx: a component's named props take the rest from `{..base}`, Rust's struct update, not JSX's `{...base}`, which would override them
```

A component takes one spread. Its `rest` and flattened props spread onto
an element as `{...rest}`: see [components](components.md#passing-dom-attributes-through).

## Components

A capitalized tag is a component: a function of the module, an imported
one, or a path to one.

```rust
// src/lib.rs
mod ui;

use react::{JSX, jsx};

pub fn Paths() -> JSX::Element {
    jsx! {
        <>
            <ui::Card title="Hi">{"inside"}</ui::Card>
            <ui::Pair::<u32> left={1} right={2} />
        </>
    }
}
```

```rust
// src/ui.rs
use react::{JSX, ReactNode, jsx};

pub struct CardProps<'a> {
    pub title: &'a str,
    pub children: &'a dyn ReactNode,
}

pub fn Card(CardProps { title, children }: CardProps) -> JSX::Element {
    jsx! {
        <section>
            <h2>{title}</h2>
            {children}
        </section>
    }
}

pub struct PairProps<T> {
    pub left: T,
    pub right: T,
}

pub fn Pair<T: ReactNode>(PairProps { left, right }: PairProps<T>) -> JSX::Element {
    jsx! { <span>{left}{" / "}{right}</span> }
}
```

```jsx
// src/lib.jsx
import { Card, Pair } from "./ui.jsx";

export function Paths() {
  return (
    <>
      <Card title="Hi">inside</Card>
      <Pair left={1} right={2} />
    </>
  );
}
```

- **A module path** is Rust's, `<ui::Card>`, closed by `</ui::Card>`. A
  renamed import, `use ui::Card as Panel;`, is `<Panel>`.
- **Generic components** infer their type arguments from their props.
  Where they can't, write Rust's turbofish, `<ui::Pair::<u32> .. />`.
- **`key`** goes on any tag and is never a prop.
- **Props, keys, spreads and children are evaluated once each, in the
  order written**, as JS evaluates JSX.
- **A component value**, a `memo`, `lazy` or `forward_ref` component or a
  context, is a `thread_local!` used as a tag, `<FAST_ROW id={1} label="one" />`:
  see [the rest of React's API](api.md). For one whose props type JSX
  can't see, an imported value say, write its props whole,
  `<Selected {...props} />`, or `{...()}` for none.

### A tag that's a value

A capitalized local of a type that implements `react::Tag`, a fieldless
enum whose variants are named as tags are, renders as that tag, as JSX's
`<Comp>` of `const Comp = "h1"` ([ADR 0220](../../decisions/0220-jsx-tag-values.md)):

```rust
use react::{JSX, ReactNode, Tag, jsx};

#[derive(Clone, Copy)]
pub enum Level {
    #[cfg_attr(rust_js, rust_js::name = "h1")]
    H1,
    #[cfg_attr(rust_js, rust_js::name = "h2")]
    H2,
}

impl Tag for Level {}

pub struct HeadingProps<'a> {
    pub level: Level,
    pub children: &'a dyn ReactNode,
}

pub fn Heading(HeadingProps { level: Comp, children }: HeadingProps) -> JSX::Element {
    jsx! { <Comp className="heading">{children}</Comp> }
}

pub fn Title() -> JSX::Element {
    jsx! { <Heading level={Level::H2}>{"Intro"}</Heading> }
}
```

```jsx
export function Heading({ level: Comp, children }) {
  return <Comp className="heading">{children}</Comp>;
}

export function Title() {
  return <Heading level="h2">Intro</Heading>;
}
```

`react::ElementType` holds any tag or a `memo` component of an element's
props, as @types/react's `ElementType` does ([ADR 0234](../../decisions/0234-element-type.md)).

## Fragments

`<>..</>` groups children without an element. A list item of several
elements is a keyed `<Fragment>`:

```rust
# use react::{JSX, jsx};
#
# pub struct Item {
#     pub id: u32,
#     pub name: String,
# }
#
# pub struct ListProps<'a> {
#     pub items: &'a [Item],
# }
#
pub fn Keyed(ListProps { items }: ListProps) -> JSX::Element {
    jsx! {
        <dl>
            {items
                .iter()
                .map(|item| jsx! {
                    <Fragment key={item.id}>
                        <dt>{item.id}</dt>
                        <dd>{item.name.clone()}</dd>
                    </Fragment>
                })
                .collect::<Vec<_>>()}
        </dl>
    }
}
```

```jsx
import { Fragment } from "react";

export function Keyed({ items }) {
  return (
    <dl>
      {items.map((item) => (
        <Fragment key={item.id}>
          <dt>{item.id}</dt>
          <dd>{item.name}</dd>
        </Fragment>
      ))}
    </dl>
  );
}
```

`<>` takes no attributes: `a fragment is <>children</>`.

## React's built-in components

`Fragment`, `StrictMode`, `Suspense`, `Profiler`, `Activity` and
`ViewTransition` are tags with no import, each imported from `react` in
the JS where it's used. Each is there from the React release that has it:
`Activity` from 19.2, `ViewTransition` and a `Fragment`'s `ref` from 19.3.
On an older React, using one is a compile error. See
[the rest of React's API](api.md#built-in-components).

A context is a tag too: `<THEME value="dark">` on React 19,
`<THEME.Provider value="dark">` on any, and `<THEME.Consumer>` with a
function child. See [context](api.md#context).

## One way to make elements

`jsx!` is how elements are made. The builders it expands to are the
compiler's, hidden from the crate's documentation, and calling one is an
error:

```text
error: rust-js: element builders are compiler-only; construct elements and set their props inside `jsx! { <Tag ... /> }`
```

Hooks, styles, contexts and React DOM's functions are ordinary Rust APIs.

## Errors

What's wrong in JSX is a compile error at the tag or attribute, and the
previous JS is left as it was. The syntax's own:

| Error | What to write |
|---|---|
| `wrap adjacent JSX elements in <>...</>` | One root, a fragment around siblings |
| `expected a literal or a Rust expression in {braces}` | Text in quotes, `{"Hello"}` |
| `` `on_click` is `onClick` in JSX, as React names it `` | The attribute's JSX name |
| `` missing prop `subtitle` of `CardProps` `` | Every prop that isn't an `Option` or defaulted |
| `expected </div>, found </span>`, `missing closing tag </div>` | Matching tags |
| `duplicate attribute` | One of each |
| `only one props spread is supported` | One spread |
| `put the props spread last` | A component's spread after its named props |
| `children were provided twice` | Children between the tags or `children={..}`, not both |
| `generic arguments belong on a function component` | Turbofish on a function component only |
| `JSX nesting exceeds 128 elements` | A component for part of the tree |

A missing field is reported at the props struct, with the JSX that left it
out as a second label.

## Formatting

rustfmt leaves a `jsx!` body as it is. The compiler's formatter indents
it: tags, props, nested JSX and expression blocks aligned at four spaces,
after rustfmt has formatted the Rust around them. It changes only leading
whitespace: line breaks, literals, comments and other macros' bodies stay
as written, and `#[rustfmt::skip]` on an item or a `jsx!` leaves it alone.
In this repository, `bun run fmt` runs both, on the workspace and the
playground or on the files it's given, and `bun run fmt:check` checks.

## What holds it

`test/jsx.test.ts` compiles each form and checks the whole module against
[`test/snapshots/jsx`](../../../test/snapshots/jsx), renders it with React,
and checks the source maps. A new form of syntax extends it first.

| Snapshot | Forms |
|---|---|
| `grammar` | Tags, fragments, text, numbers, booleans, unit, `Option`, tuples, lists, conditional and block children, comments, attributes, styles, raw HTML |
| `modules` | Components, imported aliases, keyed list items, optional children, spreads and struct updates |
| `spreads-and-paths` | DOM spread precedence, children after a spread, `::` paths, component values, keyed fragments |
| `component-kinds` | Generic components, zero-prop components, component values, `memo`, `lazy`, `forward_ref`, contexts |
| `builtins` | `Profiler`, `ViewTransition`, `Suspense`, `Activity`, `StrictMode`, refs, form actions |
| `refs` | Forwarded refs, components' `ref` props, handle types, evaluation order |
| `react18` | React 18's provider and ref forms, and what newer versions gate |
| `nested-expressions` | JSX in props, closures and macros |
| `evaluation-order` | Attributes, children and handlers in written order, their source lines |
| `keys` | Component keys, evaluated once |
| `import-collisions` | Imports that share a name, renamed |
| `svg` | SVG tags and attributes |
| `many-children` | Forty siblings |
| `module-resolution` | Nested, inline and `#[path]` modules |

Run `bun run test:snapshots`; after an intended change, `bun run bless`,
read the diff, and run them again.
