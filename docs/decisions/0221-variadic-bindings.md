# 0221. A binding's last slice can be its rest arguments

Status: Accepted. Extends [0028](0028-js-module-imports.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

Many JS functions take any number of arguments: `Math.max(a, b, c)`,
`classList.add(...names)`, and the `classnames` package react.dev calls
`cn` on every page, `cn("mdx-heading", className)`. Rust has no variadic
functions, so a binding took a fixed number, one function per count, or an
array, `cn(["mdx-heading", className])`, which isn't the JS a person
writes, nor always what the function does with it.

## Decision

**A binding marked `#[rust_js::variadic]` takes its last parameter, a
slice, as JS's rest arguments: a slice written out at the call is the
arguments themselves, and another one is spread.**

```rust
unsafe extern "Rust" {
    #[link_name = "classnames#default"]
    #[cfg_attr(rust_js, rust_js::variadic)]
    pub safe fn cn(classes: &[Option<&str>]) -> String;
}

cn(&[Some("mdx-heading"), class_name])
cn(&classes)
```

```js
cn("mdx-heading", className)
cn(...classes)
```

- **One whose last parameter isn't a slice is an error** at its call.
- **A spread is a call's argument too**, `new X(...items)` and `f?.(...items)`
  as well as `f(...items)`, where it was only an array's item (ADR 0216).

## Why

- **It's the call the JS writes**, `cn("a", className)`, as Scala.js's
  `js.Any*` and ReScript's `@variadic` give it.
- **It's Rust's**: a slice, checked, of the items' type; a caller who has
  one, a `Vec`, gives it as it is.
- **It's tested**: a compiler test calls a variadic `Math.max` with a slice
  written out and with a `Vec`, and checks the JS and what it returns; one
  whose last parameter isn't a slice is refused.

## Amendment: a binding that skips what's falsy

`#[rust_js::skips_falsy]` marks a function that skips its falsy arguments,
as classnames does. An argument shown only if a test holds,
`expanded.then_some("wide")`, is given as `expanded && "wide"`, where it was
`expanded ? "wide" : undefined`: such a function skips `false` as it does
`undefined`, and that's how react.dev writes `cn('a', isExpanded &&
'sp-layout-expanded')`.
