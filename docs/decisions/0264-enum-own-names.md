# 0264. A variant's own name is the variant

Status: Accepted. Extends [0013](0013-fieldless-enums.md) and
[0233](0233-match-reads-a-table.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev's Page names an image after its section, a string:

```tsx
`/images/og-${section}.png`
```

In Rust a section is an enum, and its name a method, as Rust writes one:

```rust
impl Section {
    pub fn as_str(self) -> &'static str {
        match self {
            Section::Learn => "learn",
            Section::Blog => "blog",
        }
    }
}
```

A fieldless variant is already its name in JS (ADR 0013), but rust-js
made the method a conditional and called it, `Section.as_str(section)`.

## Decision

- **A `match` giving each variant its own name is what's matched**: the
  method is `return section;`, as a value or returned, as ADR 0233's
  table read now is returned too.
- **A call of the crate's function whose body is such a `match` of its
  one parameter is its argument**: `section.as_str()` is `section`.

```js
return `/images/og-${section}.png`;
```

A variant given another name, `Section::Blog => "news"`, is a conditional
and a call, as before.

## Why

- **No program can tell**: each arm gives the string the variant already
  is, and a call of the function gives back what it's given, with no
  effect of its own.
- **It's the JS a person writes**, from Rust's own idiom, which runs
  natively too.
- **It's tested**: a compiler test names a section both ways, its own name
  and another; mutations keep the conditional, the returns, the call, and
  take another name for its own.
