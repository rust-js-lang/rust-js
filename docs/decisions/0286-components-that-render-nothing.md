# 0286. A component that sometimes renders nothing returns an `Option`

Status: Accepted. Extends [0236](0236-jsx-element.md) and
[0201](0201-components-take-no-dictionaries.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A component may render nothing: react.dev's DownloadButton returns `null`
where the browser can't run the sandbox it would download. In Rust that
function returns `Option<JSX::Element>`, `None` where it renders nothing,
and `jsx!` didn't take it: a component was a function returning an
`Element`, so `<DownloadButton />` named no macro, `memo(DownloadButton)`
had no `ComponentType`, and a generic one was given a dictionary React
never passes.

How the others do it (checked in local clones):

- **@types/react** types what a function component returns as
  `ReactNode`, which takes `null` and `undefined`; React 18 and later
  render either as nothing.
- **ReScript** returns a `React.element` and writes `React.null` for
  nothing: one type, a value standing for none.

## Decision

**A component returns `JSX::Element` or `Option<JSX::Element>`, which is
the element, or `undefined` where it's `None`, as every `Option` is.**

```rust
pub fn Label(LabelProps { text, shown }: LabelProps) -> Option<JSX::Element> {
    if !shown {
        return None;
    }
    Some(jsx! { <b>{text}</b> })
}
```

```jsx
export function Label({ text, shown }) {
  if (!shown) {
    return undefined;
  }
  return <b>{text}</b>;
}
```

- react's `ComponentType` takes a function returning either, through a
  trait of what a component renders, `Rendered`.
- `jsx!` gives one its props macro, as it does one returning an `Element`,
  and rust-js gives it no dictionary and no drop, as any component (ADRs
  0199, 0201).

## Why

- **Rust's way to say "maybe nothing"**: a port's `return null` is
  `return None`, which reads as the original does.
- **`undefined`, not `null`**: an `Option` is `undefined` where it's
  `None` everywhere else in rust-js, and React renders it the same.

## Alternatives

- **An empty element**, `jsx! { <></> }`: a fragment in the output where
  the original has none.
- **Any `ReactNode`**, as @types/react: a string or a number as a
  component's result, which no port has needed, and which would make every
  capitalized function a component to `jsx!`.

## Consequences

- A component of `Option<JSX::Element>` is used, memoized and declared as
  one of `JSX::Element`; its declaration says `JSX.Element | null |
  undefined`, as every `Option` a function returns.
