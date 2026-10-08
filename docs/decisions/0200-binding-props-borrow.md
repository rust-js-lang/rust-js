# 0200. A binding's props borrow their text, and take `aria-label` and the rest

Status: Accepted. Amends [0192](0192-next.md); extends [0194](0194-jsx-attributes-read-once.md)
and [0195](0195-rest-props.md).

## Context

react.dev's `ButtonLink` is a `next/link` `Link` with classes it computes,
`cn(className, ...)`, its `label` as the link's `aria-label`, and the props
it isn't given by name, `{...props}`. The `next` crate's `LinkProps` could
take none of them:

- **Its text was `&'static str`**, which a `String` a component joins
  isn't.
- **`aria-label` was no field**, nor could be: `jsx!` gave a component's
  `aria-label` to the field `r#aria-label`, which Rust can't parse.
- **It had no `Rest`**, which a component's own props pass on (ADR 0195).

And what it wrote of such a link, its props before its children, copied
each variable to another first, `const rest$1 = rest;`, as an element's
attributes no longer were (ADR 0194).

## Decision

**A binding's props struct borrows its text, `LinkProps<'a, C>` with
`class_name: Option<&'a str>`, and names what an element is given, its
`aria-label` too, and a `rest: Rest` for the rest:**

```rust
let classes = [class_name, "link"].join(" ");
jsx! {
    <Link href="/" className={Some(classes.as_str())} aria-label={label} rest={rest} {..Default::default()}>
        {"Home"}
    </Link>
}
```

```jsx
const classes = [className, "link"].join(" ");
return (
  <Link href="/" {...rest} className={classes} aria-label={label}>
    Home
  </Link>
);
```

- **Since ADR 0208, `next/link`'s rest is React's anchor attributes**,
  `#[rust_js::flatten] anchor: AnchorHtmlAttributes<'a>`, as Next.js types
  them, where a `Rest` was: a component passes its own on, `anchor={props}`.
- **Its `rest` is where it's written**: before the props named after it,
  which take its place, as a JS component's `{...props}` before its own
  `aria-label` (ADR 0203).
- **A component's `aria-label` is the field `aria_label`**, `-` as `_`,
  which `rust_js::name = "aria-label"` names again in the JS.
- **`next/image`'s `ImageProps<'a>` borrows its text too.**
- **A component's prop read from a variable nothing writes again is read
  as it is**, before its children too, as an element's attribute is.

## Why

- **It's what a JS caller gives**: a string it computed, an `aria-label`,
  and the rest of its own props, and what it writes, `className={classes}`,
  is what a person would. A component given borrowed props keeps nothing
  of them: React's element holds the JS values.
- **It's tested as a Next.js app is built**: `test/next.test.ts`'s
  `/about` links home by a component of its own, which gives `Link` its
  computed classes, an `aria-label` and its `Rest`, and checks the page
  Next.js renders, `class="home link"` and `aria-label="Home page"`, and
  the JSX, `{...rest}`, no copy of it.

## Since

- **A component of the crate's own reads one by its name quoted**,
  `function Kbd({ "data-platform": dataPlatform })`, as react.dev's TopNav
  has it; a JSX test renders one, and a mutation prints the name bare.
