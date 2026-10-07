# 0228. A tag takes the attributes @types/react gives it

Status: Accepted. Extends [0224](0224-typed-intrinsic-elements.md), which left
this for later, and [0043](0043-react-versions.md).

## Context

TypeScript's `JSX.IntrinsicElements` says what each tag takes:
`button: DetailedHTMLProps<ButtonHTMLAttributes<HTMLButtonElement>, HTMLButtonElement>`.
A `<div href="/a">` is TypeScript's error, as `HTMLAttributes` has no
`href`. In rust-js every tag took every attribute React knows, so it
compiled, and React wrote an `href` on a `<div>`.

## Decision

**An attribute every element takes, `HTMLAttributes`' (`className`, `id`,
`title`, …), is every tag's. Another is a method of an element whose tag
@types/react gives it, by a marker trait: `.href(..)` is `where T:
has::Href`, which `HTMLAnchorElement`, `HTMLAreaElement`, `HTMLBaseElement`,
`HTMLLinkElement` and `HTMLStyleElement` have.**

```text
error[E0277]: `react::webapi::HTMLDivElement` takes no `href`
  |     jsx! { <div href="/a" /> }
  |                 ^^^^ not an attribute of this tag
  = note: @types/react gives `href` to <a>, <area>, <base>, <link>, <style>
```

- **`react/generate.ts` reads it from @types/react**: each tag's attribute
  interface in `JSX.IntrinsicElements`, with what it extends, and each tag's
  element as webapi's `Tag` gives it. A trait, `react::has::Href`, is
  implemented for each element of a tag that takes the attribute.
- **webapi binds every HTML element interface** of the spec but the
  obsolete ones, so each tag has its own element, `<video>` an
  `HTMLVideoElement`, where 82 tags were an `HTMLElement`; 39 that are one
  in the DOM, `<b>`, `<section>`, still are, and take what the tags of an
  `HTMLElement` take.
- **Any `Element` takes every attribute**: a tag of no element of webapi's,
  an SVG one's, and a tag value, `<Comp href=..>` of a `react::Tag` that may
  be an `<a>` (ADR 0220), as TypeScript's are any intrinsic's.
- **Each attribute takes what @types/react types it as**, in every
  interface that has it: `className={..}` text (`value::Text`),
  `tabIndex` a number (`value::Number`), `width` either
  (`value::NumberOrString`), `draggable` a `bool` or text
  (`value::Booleanish`), `disabled` a `bool`; each an `Option` of one too.
  Text for a number is an error that says so, ``` `str` is not a number ```.
  One @types/react doesn't type, `download`'s `any` or one React DOM's
  table alone has, takes any `Value`, as each did, a string, number or
  `bool`, but a hand-written list's `bool`s. (Amended.)
- **Event handlers stay every element's**, as `DOMAttributes` has them.
- **An attribute no interface types**, one React DOM's table alone has,
  stays every element's.

## Why

- **It's TypeScript's check**, from the same source, @types/react, so it
  holds no more and no less than TypeScript does.
- **The error says what's wrong**, the tag and the attribute, and which
  tags take it (`#[diagnostic::on_unimplemented]`).
- **It's tested**: a JSX test's `<a href>`, `<button disabled>`, `<input
  value>`, a global `title` and a tag value's `href` compile and render; a
  `<div href>` and a `<span disabled>` are errors that say so.

## Costs

- **Tags of one element share its attributes**: `<td>` and `<th>` are both
  an `HTMLTableCellElement`, so each takes the other's `scope`.
- **A form's `action` and a button's `formAction`**, written by hand, are
  still every tag's.
