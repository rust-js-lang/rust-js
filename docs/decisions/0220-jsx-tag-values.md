# 0220. A JSX tag can be a value, named by a capitalized local

Status: Accepted. Extends [0040](0040-jsx.md) and [0213](0213-props-as-written.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

JSX reads a capitalized variable as what to render, and a string there is a
DOM element's tag: react.dev's `Heading` is a `<div>`, or the `<h1>` to
`<h5>` its `as` prop names, with every other prop spread onto it:

```jsx
function Heading({ as: Comp = "div", className, children, id, ...props }, ref) {
  return <Comp id={id} {...props} ref={ref} className={cn("mdx-heading", className)}>..</Comp>;
}
```

`jsx!` had no such tag. A capitalized name was a component, built through
its props companion, `Comp!`, which a local doesn't have; and a spread had
to come last, where the original's `className` comes after it.

## Decision

**A fieldless enum whose variants are named as tags is a `react::Tag`, and
`jsx!` reads a tag that names a capitalized parameter or `let` of its
function as an element of it, which takes what a DOM element takes, a
spread among its attributes in their order:**

```rust
#[derive(Clone, Copy, Default)]
pub enum As {
    #[cfg_attr(rust_js, rust_js::name = "h1")]
    H1,
    #[default]
    #[cfg_attr(rust_js, rust_js::name = "div")]
    Div,
}
impl react::Tag for As {}

fn Heading(HeadingProps { r#as: Comp, id, children, rest, .. }: HeadingProps<..>) -> Element {
    jsx! { <Comp id={id} {...rest} className={..}>{children}</Comp> }
}
```

```jsx
function Heading({ as: Comp = "div", id, children, ...rest }) {
  return <Comp id={id} {...rest} className={..}>{children}</Comp>;
}
```

- **The names are the function's own**, its parameters' and `let`s'
  bindings that start with an uppercase letter, a field's pattern
  `r#as: Comp` among them; a function inside has its own. The formatter
  reads them as `jsx!` does.
- **It's `react::tag(Comp)`**, then the attributes a DOM element's are,
  which rust-js writes as JSX's `<Comp ..>`: a tag that's a variable.
- **A local given only a spread, `<Comp {...rest} />`, is a component's
  call, as before**, `let Selected = ui::Empty;` one: a `Tag` is a
  `Component` of any props too, so a tag takes them so as well.
- **A DOM element's spread may come before attributes**, as JSX's may,
  each written where it is, so what comes later wins, as in JSX. A
  component's spread still comes last, its named props from `{..base}`
  (ADR 0213).

## Why

- **It's the JSX the original writes**, `{ as: Comp = "div" }` and
  `<Comp ..>`, with the props it spreads where it spreads them.
- **It's checked**: `Comp` is a `Tag`, so not any value, and what it takes
  is a DOM element's attributes, each of its type.
- **It's tested**: a JSX test renders a `Heading` of a prop's tag, its
  default and a caller's, and of a `let`, and checks the JSX; the
  formatter lays out a tag's attributes after its spread; a component's
  attribute after a spread is still refused.

## Costs

- **A capitalized local hides a component of its name** in that
  function's JSX.
- **What a tag takes isn't narrowed to its element's**: an `<h1>` and a
  `<div>` take the same attributes, as React's own types give them.

## Amendment: a string is a tag, and a renderer's host type one of its props (rd331d)

Decided by the user (2026-11-01), the two forms recommended for
react-art's ReactART.js, whose tags are strings read at run time:

```js
const Tag = Mode.Surface.tagName;             // "canvas" or "svg", React DOM's
return <Tag ref={ref => (this._tagRef = ref)} accessKey={props.accessKey} .. />;

const T = TYPES.TEXT;                         // "Text", ART's own renderer's
return <T {...this.props} ref={t => (this._text = t)}>{..}</T>;
```

Case: N ([0262](0262-when-rust-and-js-disagree.md)): the tag is the
string either way, which React renders as it renders any.

- **A `&'static str` is a `react::Tag`**, a DOM element's of any name, as
  TypeScript's `string` tag is and React DOM's `createElement(type)` takes
  one: `let Tag = mode::surface_tag_name(); <Tag ref={..} accessKey={..}
  ..>`, its attributes a DOM element's, `webapi::Element`'s, which takes
  `accessKey`, `draggable` and `title` already, as HTML's elements do,
  and its `ref` an `Element` or what it extends. This loosens the
  Decision's "not any value": a string is a tag, which TypeScript's JSX
  checks no closer.
- **A renderer's host type is `react::HostType<P, I>`**, a string in JS
  typed by the props `P` its element is given and the instance `I` its
  `ref` is, as TypeScript's custom renderers type their intrinsic
  elements, made `const` by `HostType::new`, the string itself: `pub
  const TEXT: HostType<TextProps, Node> = HostType::new(TYPES.text);`,
  `export const TEXT = TYPES.TEXT;`. A capitalized local of one is a tag
  `jsx!` reads as it reads any, `react::tag(T)`, whose element while its
  props are set is a `HostElement<P, I>`: its props whole, `{...props}`
  of a `P` or a reference to one, `{...self.props()}`; a `ref` of `I`; a
  `key`; children. No DOM element's attribute or event is one of its
  methods, which ART's renderer doesn't take; a prop written by name is
  rustc's error, `no method named ..`, as the renderer is given its props
  whole. Given its props alone, `<T {...props} />`, it's a component's
  call, as a `Tag` is (above). rustc checks both: `P` and `I` are what
  the element is given.
- **What `react::tag` makes is the tag's**, `TagElement::Element`: a
  `Tag`'s an `Element`, a `HostType`'s a `HostElement`, each made an
  `Element` by `react::element`, as an intrinsic's is (ADR 0224).
- **A `const` of a binding's JS value is its initializer's**: rustc's
  value of `HostType::new("Text")` is its markers, nothing of the string,
  so it's `HostType::new`'s `"Text"`, as a static's `ElementType` is (ADR
  0234).

Rejected: one form for both, a `Tag<I>` of the instance its element has,
which would give a renderer's tag the DOM's attributes and events,
`onClick(SyntheticEvent<Node>)`, wrong for ART's; and only
`ElementType::from_unknown(js::unknown(&tag))`, a cast that types the ref
as a DOM `Element` where ART's is its `Node`.

Held by `test/class-components.test.ts`'s "a tag that's a string: a DOM
element's, and a host type's of its props and instance", rendered by
React DOM, a ref of each its instance; and the mutations
`host-type-element-unfinished` (`jsx_syntax/parser.rs`) and
`const-of-a-js-value-its-markers` (`lower/representation.rs`).
