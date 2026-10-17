# 0443. A value a caller may leave out is an `Option`, in each direction

Status: Proposed. The design we want, before its work: once delivered, it
would supersede the `_with_` forms of an optional argument that
[0024](0024-web-crate.md), [0102](0102-js-and-webapi.md) and
[0272](0272-node.md) give, and extend [0196](0196-typescript-declarations.md),
[0313](0313-console-forms.md) and [0330](0330-undefined-given-to-js.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)): whether a value left
out is written out or not changes nothing a program does; and least
surprise and "Exactly one way" (AGENTS.md).

## Context

JS lets a caller leave a value out: a parameter, `pushState(data, unused,
url?)`, or an object's field, `{ href?: string }`. TypeScript writes both
`x?: T`. Rust has one way to say "this may be absent", `Option<T>`, and no
optional parameters or overloads, so each binding chose a shape, and they
differ:

- **An object's field, a prop, an options object** is an `Option`, `None`
  left out, `?:` in a declaration (ADRs 0102, 0196, 0438). One rule, held.
- **A JS function's optional parameter, called from Rust**, is a function
  of its own, as wasm-bindgen's web-sys names them, `_with_<argument>`
  (ADRs 0024, 0102, 0272): `push_state` and `push_state_with_url` for one
  `pushState`, four for `addRule(selector?, style?, index?)`, up to
  `add_rule_with_selector_and_style_and_index`. webapi has 787 such names;
  the hand-written crates mix both shapes, `react::memo_with` and
  `create_root_with` beside functions of the js and next crates that take
  an `Option`:

  | Crate | `_with` forms | an `Option` parameter |
  |---|---|---|
  | webapi (generated) | 787 | few |
  | js (`builtins/`) | 34 | 9 |
  | react | 31 | 1 |
  | next | 28 | 8 |
  | node | 123 | 1 |

- **A Rust function's `Option` parameter, called from JS**, is declared
  `name: string | null | undefined`, a parameter TypeScript makes its
  caller give, `shown(undefined)`, where `shown()` is what a person writes.

So one concept, a value a caller may leave out, had three shapes, and
which a binding has can't be guessed: it's learned name by name.

## Decision

**A value a caller may leave out is an `Option` in Rust and `?` in
TypeScript, wherever it crosses, in each direction.** `None` is absent,
`Some(x)` present, and a trailing absent value is no argument.

| Where | TypeScript | Rust |
|---|---|---|
| A JS function's parameter, a binding's | `pushState(data: any, unused: string, url?: string \| URL \| null)` | `push_state(data, unused, url: Option<..>)` |
| An object's field, a prop, an options object | `{ href?: string }` | `href: Option<&str>` |
| A Rust function's parameter, JS's to call | `greet(name: string, greeting?: string \| null)` | `greet(name: &str, greeting: Option<&str>)` |
| The JS of a call that leaves one out | `history.pushState(data, "")` | `history.push_state(data, "", None)` |

1. **A binding has its TypeScript signature**: one Rust function for each
   JS function, each parameter in TypeScript's order, an optional one an
   `Option`. Its name is the JS name's, never a `_with_<argument>` form of
   it. webapi's generator writes each so; the hand-written crates' are
   rewritten so.
2. **An optional parameter whose type is generic**, a union (`impl IntoX`,
   ADR 0229) or `any` (a type parameter, ADR 0225), is taken by
   reference, `Option<&dyn IntoX>`, so a bare `None` needs no type rustc
   can't infer, `None::<f64>`; `Some(&true)` passes a member as it is.
   rust-js passes its value to JS as it is. 85 of webapi's are so.
3. **A trailing `None` given an optional parameter is no argument**, in a
   call of a binding as in one of the crate's own function:
   `history.pushState(data, "")`. TypeScript's `x?: T` takes `undefined`
   as left out, as WebIDL does an optional argument's. An `undefined`
   given a parameter that isn't optional is still given (ADR 0330),
   `Math.max(1, undefined)`; a `None` of a parameter whose `None` is
   `null`, `#[rust_js::nullable]`, is `null` (ADR 0275).
4. **A Rust function's trailing `Option` parameters are declared optional**,
   `greeting?: string | null`, so a JS or TypeScript caller leaves them
   out, `greet("Ada")`.
5. **What stays separate is what TypeScript separates**: a method it
   declares as several signatures, `drawImage(image, dx, dy)`,
   `drawImage(image, dx, dy, dw, dh)` and the nine-argument one, keeps a
   name for each, as its declaration lists them.

```rust
history.push_state(state, "", None);
history.push_state(state, "", Some("/inbox"));
sheet.add_rule(Some("p"), Some("color: red"), None)?;
canvas.to_data_url(None, None)?;
```

```js
history.pushState(state, "");
history.pushState(state, "", "/inbox");
sheet.addRule("p", "color: red");
canvas.toDataURL();
```

### Held

- A test fails where a binding crate has a function that's another's
  `_with_` form of an optional argument, naming the one function to use.
- `test/declarations.test.ts` holds a Rust function's trailing `Option`s
  declared `?`; a lowering test holds a binding's trailing `None` left out,
  and `Math.max(1, undefined)`'s kept.

### Not decided here

- **A callback JS calls with more arguments than it needs**, `map`'s
  `(value, index, array)`: there JS decides what's given and the Rust
  closure what it takes, so it isn't an optional parameter of ours. The
  js crate's `map` and `map_with_index`, `filter_with_index` and the like
  are a decision of their own; Rust's `iter().enumerate()` is a candidate
  for it.

## Why

- **Guessable from one example**: a binding reads as its TypeScript
  declaration, so MDN, or the types, tell what to write. Today it's a
  name to look up, `push_state_with_url` or `push_state_with_unused_and_url`.
- **One way**: one shape for one concept, at each crossing, in every
  crate, where there were three, mixed within one crate.
- **It's Rust's**: `Option` is how Rust says "may be absent", in a
  function's parameter as in a struct's field, the Rust a server is
  written in.
- **The JS reads as written**: a value left out is no argument, as a
  person leaves one out.

## Alternatives

- **Keep the `_with_` forms**: no `None` at a call site, but several Rust
  names for one JS function, each to look up, and a second shape beside
  the `Option` fields and the crates that take an `Option`.
- **`Option<impl IntoX>` and `Option<T>` of a generic parameter**: one
  form, but its `None` needs a type, `None::<f64>`, which nobody guesses.
- **An options struct or a builder for each function**: idiomatic for a
  Rust library's own API, but not TypeScript's signature, and a type of
  its own for each of thousands of functions.

## Consequences

- **A breaking change** of every binding crate's optional-argument forms
  and their callers: the examples, the playground, the pilot and the
  tests are migrated with it. A program that called a `_with_` form calls
  the function it's a form of, with its arguments' `Option`s.
- A call writes the `None`s a TypeScript call leaves out; the JS doesn't.
- ADRs 0024, 0102 and 0272 are marked superseded in this respect once it's
  delivered, and this one Accepted.
