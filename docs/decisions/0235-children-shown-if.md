# 0235. A child shown only if a test holds is `test && child`, where the test can't render

Status: Accepted. Builds on [0040](0040-jsx.md).

Case: A, C ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev writes a child shown only when a test holds with `&&`, some 57
times in its components: ConsoleBlock's `{level === 'warning' &&
<IconWarning />}`, ExpandableCallout's `{variant.Icon && <variant.Icon />}`.
rust-js wrote each `bool::then` or `Option::map` child as a conditional:

```jsx
{level === "warning" ? <IconWarning /> : undefined}
{variant.Icon != null ? <variant.Icon className={..} /> : undefined}
```

`&&` gives back its left side when that's falsy, and React renders what it
gets: `false`, `null` and `undefined` render nothing, but `0` and `NaN`
render as text, and `""` is a text node, which React Native refuses. So a
`&&` is the same as the conditional only where the test is never one of
those.

## Decision

**In JSX's children, `test ? child : undefined` is `test && child` where
the test is a boolean by its shape, a comparison, `!`, a `bool`, or `&&`
and `||` of them; and `x != null && child` is `x && child` where the child
maps an `Option` of a JS object, which is never falsy when it's there.**

```jsx
{level === "warning" && <IconWarning />}
{variant.Icon && <variant.Icon className={..} />}
```

- **Text and numbers keep their test**: `name != null && <b>{name}</b>` of
  an `Option<&str>`, as `""` would render, and `0` of a number; a test of
  text by its truthiness, `href ? <a /> : undefined` (ADR 0232), keeps its
  conditional.
- **A `bool` is one whatever its shape**: `is_lead.then(..)` of a
  variable is `isLead && <span />`, as react.dev's TeamMember has it.
  (Amended: it was a comparison, `!`, or a literal only.)
- **Every child is one**: `jsx!`'s children are a tuple of tuples, each
  child its own.
- **Only children**: elsewhere `false` isn't `undefined`, so a value keeps
  its conditional.

## Why

- **It's the JS a person writes**, as react.dev's.
- **It renders the same**: what the test is when it fails renders nothing,
  as `undefined` does.
- **It's tested**: a JSX test renders a `bool::then` child, of a
  comparison and of a variable, and children of
  an `Option` of text, of a number and of text tested by its truthiness,
  each empty and missing, and checks which are `&&`; another an `Option`
  of an `ElementType`.
