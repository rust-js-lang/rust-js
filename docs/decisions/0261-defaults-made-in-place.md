# 0261. Defaults that do nothing don't move what's made beside them

Status: Accepted. Extends [0201](0201-components-take-no-dictionaries.md) and
[0213](0213-props-as-written.md).

## Context

react.dev's TopNav gives its `Logo` classes inside next/link's `Link`:

```tsx
<NextLink href="/" className="..">
  <Logo className={cn('text-sm me-0 ..')} />
</NextLink>
```

Logo's props are a flattened `SVGAttributes`, so its `className` is
`SVGAttributes { class_name, ..Default::default() }`. rust-js made every
field of an update that does something first, in a `const`, so the
defaults came after it as Rust makes them, though a derived `Default`'s
do nothing. That `const` was a statement before the link's props, so the
link's own default, an object of constants, was made first too, and
spread, every one of its fields `undefined`:

```js
const anchor = { download: undefined, href: undefined, .. };
const class_name = cn("text-sm me-0 ..");
<NextLink href="/" className=".." {...anchor}>
```

## Decision

- **A field of an update waits for nothing when its defaults do
  nothing**: it's made where it's given.
- **An object made of constants is settled**: made before or after what
  follows, it's the same, so it's not made first.

```js
<NextLink href="/" className="..">
  <Logo className={cn("text-sm me-0 ..")} />
</NextLink>
```

## Why

- **It's the JSX a person writes.**
- **It's tested**: a JSX test gives a child's flattened prop by a call,
  and by statements; mutations make the first wait for its defaults, and
  the second's link default first.
