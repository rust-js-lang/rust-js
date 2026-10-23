# 0443. A value a caller may leave out is an `Option`, in each direction

Status: Accepted; amended by [0593](0593-a-lazy-argument-is-its-value.md),
whose `use_state` is one function, and [0594](0594-an-overload-an-options-dictionary-holds-is-it.md),
whose overload an options dictionary holds is that one, and [0640](0640-a-rest-parameter-is-a-tuple.md),
whose `None` before a rest of none is no argument, and
[0648](0648-v8s-stack-trace-api-is-the-node-crates.md), whose setter's
`None` is written, `x = undefined`, not left out. Amends [0024](0024-web-crate.md),
[0102](0102-js-and-webapi.md) and [0272](0272-node.md), whose `_with_` forms
of an optional argument are gone, and [0020](0020-structs-and-tuples.md):
a `()` as a `dyn` JS is given the value of is an array too. Extends
[0196](0196-typescript-declarations.md), [0313](0313-console-forms.md) and
[0330](0330-undefined-given-to-js.md). Amended by
[0593](0593-a-lazy-argument-is-its-value.md).

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
5. **What stays separate differs in a parameter or in its result**, not
   only in more of them: `drawImage(image, dx, dy, dw?, dh?)` and the
   nine-argument one, whose second is `sx`, not `dx`; `Buffer.from(string)`
   and `Buffer.from(arrayBuffer)`; `createAnswer()`, a promise of an
   answer, and its callbacks' form, a promise of nothing. Signatures that
   only add arguments are one function, even where TypeScript writes them
   apart, `stroke()` and `stroke(path)`, or WebIDL does, `alert()` and
   `alert(message)`: webapi's generator folds such overloads.
6. **A `()` as a `dyn` JS is given the value of is `[]`**, as a `()`
   argument is (ADR 0020): `use_effect(f, Some(&()))` is
   `useEffect(f, [])`, `use_effect(f, None)` `useEffect(f)`, which runs
   after every render.
7. **A generic optional parameter whose bound isn't a marker's** is an
   `Option` of it, its `None` of a type: node's
   `remove_all_listeners(None::<Ping>)`, the event's type checked by
   `Self: Emits<E>` (ADR 0361), which a `&dyn` can't be.

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

- `test/architecture.test.ts` fails where two bindings of one JS name, in
  one impl or module, give one result and one's parameters begin the
  other's, naming the one function to give an `Option` each, in every
  binding crate: whatever the second is called, `_with_url`, `_to_end`,
  `_from`, `_on_every_render`.
- `test/declarations.test.ts` holds a Rust function's trailing `Option`s
  declared `?`; a lowering test holds a binding's trailing `None` left out,
  and `Math.max(1, undefined)`'s kept.

### A callback given more than it reads

JS calls `map`'s callback with `(value, index, array)`, and a JS callback
takes what it reads; a Rust closure's arguments are its type's, so the
js crate had two functions for one JS method, `map(|x| ..)` and
`map_with_index(|x, i| ..)`. Decided with the user: **one function, its
closure given the index, as TypeScript declares it, `(value, index) =>
..`**: `a.map(|x, _| x * 2)`, `a.reduce(|sum, x, _| sum + x, 0)`. The JS
leaves out a trailing `_` it doesn't read, `a.map((x) => x * 2)`, as a
person writes it. `_with_index` forms are gone: the typed arrays',
React's `Children.for_each`, whose child is an `Option`, `None` of an
empty one, as `forEach` gives it (the form without the index said it
never was). The array argument, which no binding reads, isn't given.

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
  the function it's a form of, with its arguments' `Option`s. webapi has
  9138 functions, from 9933; 50 `_with_` names are left, each an overload
  that differs in a parameter.
- A call writes the `None`s a TypeScript call leaves out; the JS doesn't.
  The most common: `use_effect(f, Some(&[n]))`, `memo(Card, None)`,
  `window.fetch(url, None)`, `event::new("ping", None)`.
- Adding an optional parameter to a binding later is a breaking change in
  Rust, as it isn't in TypeScript: a binding has its TypeScript signature
  from the start.
- A trailing `None` given a variadic binding is still `undefined`:
  `setMaxListeners(undefined, ...targets)`, which Node reads as left out.
  (Amended by ADR 0640: before a spread; before a rest of none written
  out, `()`, it's no argument, `console.timeLog()`.)
