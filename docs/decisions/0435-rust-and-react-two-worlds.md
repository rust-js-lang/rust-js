# 0435. Rust's world and React's: each its own names, JSX the boundary

Status: Accepted. Supersedes [0046](0046-camel-case-crates.md). Extends
[0038](0038-js-names-and-destructuring.md), [0040](0040-jsx.md) and
[0213](0213-props-as-written.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)): which name a thing has
in JS changes nothing a program does, only who can guess it; and least
surprise (AGENTS.md).

## Context

A rust-js app lives in two worlds. Rust's, where a function is `use_clicks`
and a field `on_click`, as rustc's lints and every Rust reader expect. And
React's, in JavaScript, where a hook is `useClicks`, a prop `onClick`, and
React's own tools find a hook by its name. They meet at JSX: a Rust
component is rendered by React, its props given by JS, and its hooks run by
React's rules.

rust-js had no one answer to where each name lives:

- **`js::camel_case!();`, a crate's switch** (ADR 0046), made every name a
  crate defines camelCase in JS, its functions and data too. Without it, a
  component's props were Rust's in JS, `<Button on_click={..}>`, a hook
  `use_dark_mode`, which React Compiler takes for an ordinary function. The
  same Rust gave different JS by one line in `lib.rs`, and only the
  playground had the line.
- **React's own props** were camelCase only because each of their fields
  said so, `#[rust_js::name = "onClick"]`.
- **`jsx!` took two spellings of each prop**, `onClick=` and `on_click=`,
  as it made a field of either; the tests wrote both.

## Decision

**What's React's has React's names in JS; what's Rust's own keeps Rust's;
and JSX, where the two meet, is written as JSX is.** No switch: one rule,
the same for every crate.

```rust
pub fn use_clicks() -> u32 { *use_state(0u32).0 }          // a hook
pub struct ButtonProps { pub on_click: Handler, pub label_text: String }
pub fn Button(ButtonProps { on_click, label_text }: ButtonProps) -> JSX::Element { .. }
pub fn first_word(text: &str) -> &str { .. }               // Rust's own

jsx! { <Button onClick={..} labelText="Go" /> }
```

```jsx
export function useClicks() { .. }
export function Button({ onClick, labelText }) { .. }
export function first_word(text) { .. }

<Button onClick={..} labelText="Go" />
```

- **Rust is written as Rust is.** A hook is `use_clicks`, React's are
  `use_state`, `use_memo`; a props field is `on_click`. rustc's naming lints
  hold, and no crate needs `#![allow(non_snake_case)]` for them.
- **`jsx!` is JSX.** A prop is written by its name in JSX and only so:
  `onClick`, `className`, `aria-label`, `labelText`. Rust's spelling is
  rust-js's error, which says what to write:

  ```text
  error: rust-js: `on_click` is `onClick` in JSX, as React names it
  ```

  An element's is its builder's prop; a component's, its field's JS name.
  `jsx!` records each component prop as written, beside the call, and
  rust-js checks it against the props' fields, a flattened one's too.
- **In JS, React's things are camelCase** (`field_key`, `fn_name`, one rule
  each):
  - **A component's props**: the fields of a struct a component of its
    crate takes, of one its crate's JSX spreads into an element, `<div
    {...attrs}>`, and of one flattened into either. `on_click` is
    `onClick`, in the JSX, the component's own destructuring, its `.d.ts`,
    and for a JS caller.
  - **A hook**, as React defines a custom one: a function `use_x` that
    calls a hook, a binding React names `useX` or another of these.
    `use_dark_mode`, calling `use_sync_external_store`, is `useDarkMode`;
    a `use_it` that calls none is `use_it`. Another crate's is imported by
    the name its crate gave it, from its manifest.
- **Everything else is Rust's own**, its name as written: functions,
  methods, a data struct's fields, `{ first_name: "Ada" }`. A struct JS
  reads by other names, a library's options say, names each with
  `#[rust_js::name]`, as a binding does. JSON's names stay serde's either
  way.
- **`js::camel_case!` is gone**, a compile error that says so.

## Why

- **Each is guessable from one example**, in its own world: a Rust reader
  sees Rust, a React reader sees React, and JSX looks like JSX.
- **It's React's definition**: React finds a hook by its `use` name and
  calls a function that calls hooks one; a component's props are what JSX
  gives it.
- **One way**: no switch, one spelling in `jsx!`, one rule per kind of
  name, each in one place (`bindings::name_in_js`).
- **It's tested**: a React test compiles a crate with a hook, one hook
  calling another, a `use_` function that calls none, a component's props,
  a struct spread into an element and one flattened into props, and data
  and functions of Rust's names, and refuses `js::camel_case!`; JSX tests
  refuse `on_click`, `class_name` and `aria_label` on an element and
  `label_text` on a component, each with what to write. A mutation for
  each step is caught.

## Consequences

- **A crate that had `js::camel_case!();`** removes it. Its hooks and props
  keep React's names; its other functions and fields are Rust's again, so a
  hand-written JS caller of them uses those, or the item names itself.
- **`jsx!` written with Rust's names** (`on_click=`, `class_name=`) is an
  error: write JSX's.
- **A component's props are camelCase for its JS callers**: `Button({
  onClick })`, where it was `Button({ on_click })`.
- **Which structs are props is found by use**: one used as props only in
  another crate's JSX, spread there, keeps its Rust names; it can name its
  fields with `#[rust_js::name]`.
