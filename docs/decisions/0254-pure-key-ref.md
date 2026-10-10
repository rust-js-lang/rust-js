# 0254. A key or ref that does nothing is captured in no order

Status: Accepted. Extends [0203](0203-component-props-as-written.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

`jsx!` gives a component's `key` after its props are made, and a `ref`
by its own way, so where either is written before a prop that does
something, it captures every prop in a match, in written order (ADR
0203). react.dev's SidebarLink gives next/link `ref={ref}` before its
classes, which `cn` makes, so its JSX was:

```js
const match = target;
const match$1 = cn(..);
const match$2 = [<div>..</div>, ..];
return <Link href={href} ref={ref} className={match$1} ..>{match$2}</Link>;
```

## Decision

**A key or ref whose value, as written, does nothing is given where JSX
puts it, nothing captured for it: a literal, a variable or a path, a
field of one, `&` or `*` of one, or a constructor of such,
`Some(anchor)`.** One that might do something, a call, is captured as
before.

```jsx
<Tag label={described(label)} key={label} />
```

## Why

- **It runs the same**: what does nothing reads the same wherever it's
  read.
- **It's the JS a person writes.**
- **It's tested**: a JSX test keys a component by a variable before a
  prop a call makes; a mutation captures it again.

## Amendment: an import a capture reads

A key that might do something still captures the props written with it,
and one that reads an import, react's `Fragment` of Headless UI's
`as={Fragment}` in react.dev's NavigationBar, is read in place, as a
variable nothing writes again is: JS never lets a module change an
import's binding. It was a `const match = Fragment`.

## Amendment: a component's key where it was written

`jsx!` gives a key last, as `.key(k)`, and a component's was printed last:
`<Listbox.Option value={filePath} as={Fragment} key={filePath}>` where
react.dev's NavigationBar writes `key` first. A component's key goes where
it was written, among its props: `jsx!` reads each in that order already,
captured or not. One written before `{..base}`, whose spread is put first
(ADR 0250), comes before it: `<Panel key={k} {...props} wide />`. An
element's was where it was written already.

## Amendment: a key that reads alike leaves the children in place

A component's key, given last, after its children, as `.key(k)`, put
its children in `const`s first, so the key, written first, didn't read
ahead of them: `const children = <Link ..>; <LI key={item.url}>{children}</LI>`.
A key that reads the same wherever it's read, a field of a value nothing
changes (`reads_alike`), can't see what they do, nor they it, and they
stay in place, as react.dev's InlineTocItem and LanguageList write them.
One the children change, `key={counter.count}` beside
`{bump(&mut counter)}`, still has them read first. A JSX test has both;
mutations keep the children first always, and never.
