# 0228. A tag takes the attributes @types/react gives it

Status: Accepted. Extends [0224](0224-typed-intrinsic-elements.md), which left
this for later, and [0043](0043-react-versions.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

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
  and a tag value, `<Comp href=..>` of a `react::Tag` that may be an `<a>`
  (ADR 0220), as TypeScript's are any intrinsic's.
- **An SVG tag is its SVG element**, as webapi's `SVGTag` gives it (ADR
  0223): `<circle>` an `SVGCircleElement`, its ref's and its events'
  `currentTarget`; a name HTML has too, `<a>`, is HTML's, as
  `JSX.IntrinsicElements` has it. It takes `SVGAttributes`, which
  @types/react's `SVGProps` gives every SVG tag: an attribute only it has,
  `cx`, is SVG's elements' (`has::Svg`), so `<div cx>` is an error, as one
  only `HTMLAttributes` has, `hidden`, is HTML's elements' (`has::Html`),
  so `<circle hidden>` is; one both have, `className`, every element's.
  Each family is implemented through its marker, which any `Element` has.
  (Amended: an SVG tag was an `Element`, of every attribute, and SVG's
  attributes every element's.)
- **Each attribute takes what @types/react types it as**, in every
  interface that has it: `className={..}` text (`value::Text`),
  `tabIndex` a number (`value::Number`), `width` either
  (`value::NumberOrString`), `draggable` a `bool` or text
  (`value::Booleanish`), `disabled` a `bool`; each an `Option` of one too.
  Text for a number is an error that says so, ``` `str` is not a number ```.
  One @types/react doesn't type, `download`'s `any` or one React DOM's
  table alone has, takes any `Value`, as each did, a string, number or
  `bool`, but a hand-written list's `bool`s. (Amended.)
- **A literal of an attribute of a few strings is one of them**, as
  TypeScript checks it: `<img referrerPolicy="no-referer">` is ``jsx:
  `"no-referer"` isn't a `referrerPolicy`, which is one of "", "no-referrer",
  ..``, a `<button>`'s `type="sumbit"` and `aria-live="loud"` too. rust-js's
  JSX parser checks it against a table `react/literals.ts` writes from
  @types/react, `src/jsx_syntax/literals.rs`, by the tag where tags differ:
  an `<input>`'s `type` takes any string. A value that isn't a literal, a
  `&str` variable, isn't checked, where TypeScript refuses a `string` there.
  (Amended: each took any text.)
- **A CSS property takes what csstype types it as**, as @types/react's
  `CSSProperties` extends csstype's `Properties<string | number>`: one of a
  length, `width`, a number in pixels or text (`value::NumberOrString`), one
  of a number, `opacity`, `z_index`, the same, and another text only,
  `color(3)` an error (`value::Text`); 492 of webapi's 753, the rest, newer
  than csstype, any `Value`. A set of strings csstype closes, `position`'s,
  isn't checked: a style is a method's call, not JSX's literal. (Amended:
  each took any `Value`.)
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
