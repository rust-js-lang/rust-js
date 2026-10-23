# 0640. A TypeScript rest parameter is a variadic binding of a tuple

Status: Accepted. Extends [0221](0221-variadic-bindings.md), whose tuple
was an emitter's arguments; amends [0313](0313-console-forms.md), whose
`console` took one value per call, and [0443](0443-a-value-a-caller-may-leave-out-is-an-option.md),
whose trailing `None` of a variadic binding was always given.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, a binding's shape:
each call gives JS the arguments TypeScript's would.

## Context

TypeScript's DOM declares `console.error(...data: any[])`,
`el.append(...nodes: (Node | string)[])` and
`classList.add(...tokens: string[])`. webapi's generator bound each rest
parameter as one argument, `pub fn error<D>(data: D)`, `append(&self,
nodes: impl IntoNodeOrStr)`, or none, where it came after an optional
one: `console.assert(condition)`, `console.timeLog(label)`,
`setTimeout(handler, timeout)`. Narrower than TypeScript. React calls

```js
console.error(
  'The provided `%s` prop is an unsupported type %s.' +
    ' This value must be coerced to a string before using it here.',
  propName,
  typeName(value),
);
```

on every check, which no binding could write.

## Decision

**A TypeScript rest parameter is a variadic binding (ADR 0221) of a
tuple, each argument of its own type: `impl js::Rest<T>`, where `T` says
what each argument may be.** `(a, b)` is two, `(a,)` one and `()` none,
each the call's argument as it is; a tuple held is spread.

```rust
console::error((
    "The provided `%s` prop is an unsupported type %s. \
     This value must be coerced to a string before using it here.",
    prop_name,
    type_name(value),
));
el.append((child, "text"));
el.class_list().add(("open", class_name));
console::log(());
```

```js
console.error(
  "The provided `%s` prop is an unsupported type %s. This value must be coerced to a string before using it here.",
  propName,
  typeName(value),
);
el.append(child, "text");
el.classList.add("open", className);
console.log();
```

- **`T` is what one argument is**, by what it `Takes` (`js::Takes<A>`, a
  trait of `T`'s, so a crate implements it for its own `T`): `dyn
  IntoUnknown` takes any value, TypeScript's `any`, a `&str`, a `String`,
  a number, an `Option<&Unknown>`; `str` takes text, `&str`, `&String`,
  `String`; a union's `dyn IntoNodeOrStr` each member its `IntoNodeOrStr`
  takes; an interface's `MediaStream` a reference to it or to one that
  extends it. `Rest<T>` is each tuple of up to 12 whose items a `T`
  takes.
- **A tuple is the one form.** A value that isn't one is refused, by the
  trait's message: "`&str` is not a tuple of rest arguments", and rustc's
  "use a unary tuple instead", `(message,)`.
- **A `None` before a rest of none is no argument**, as ADR 0443 leaves a
  trailing one out: `console::time_log(None, ())` is `console.timeLog()`.
  Before a spread, `console.timeLog(undefined, ...held)`, it keeps its
  place; a rest's own `None` is `undefined`, given (ADR 0330).
- **The generator writes every rest parameter so**: 53 bindings, each of
  which took one argument, or none after an optional one (4): `console`'s
  11 (`log`, `error`, `warn`, `info`, `debug`, `trace`, `dirxml`,
  `group`, `groupCollapsed`, `assert`'s data, `timeLog`'s); the 18 of
  `ChildNode` and `ParentNode` (`append`, `prepend`, `replaceChildren`,
  `before`, `after`, `replaceWith` of an element, a document, a fragment,
  character data and a doctype); `DOMTokenList`'s `add` and `remove`;
  `Document`'s `write` and `writeln`; a slot's `assign`; `setTimeout`'s and
  `setInterval`'s arguments; CSS Typed OM's 14 (`CSSNumericValue`'s `add`,
  `sub`, `mul`, `div`, `min`, `max`, `equals`, `toSum`, `CSSMathSum`'s,
  `CSSMathProduct`'s, `CSSMathMin`'s and `CSSMathMax`'s constructors,
  `StylePropertyMap`'s `append` and `set`); `new Highlight(..ranges)`; and
  WebRTC's `addTrack` and `setStreams`.

## Why

- **It's TypeScript's signature**: each argument of the type an item of
  the rest parameter is, any number of them, `console.error(format, a,
  b)` as React writes it.
- **It's Rust's**: a tuple is how Rust gives values of different types
  together, and rustc checks each, where a slice of one type couldn't hold
  a format and its values.
- **It's the call a person writes**: the tuple is the arguments, no array.

## Alternatives

- **A single value or a tuple**: `console::error(x)` beside
  `console::error((x, y))`, two forms of one call; and a blanket `Rest`
  of any value overlaps a tuple's, which Rust refuses.
- **A slice, `&[&dyn IntoUnknown]`** (ADR 0221's first form): one type for
  every item, so a format and its values are each borrowed as `&dyn`,
  `&[&"a", &n]`, and an element and text can't be one `append`.
- **A function per count, `error2`, `error3`**: many names for one call.
- **A type parameter of its own per argument, as `any` is (ADR 0225)**:
  Rust has no variadic generics; a trait of tuples is how std gives them
  too, as its `Fn` traits do.

## Consequences

- A call of one is `(x,)`: `body.append((app,))`, `console::warn((msg,))`.
  The examples' consecutive appends are one call each, `app.append((input,
  view.list, footer))`, `app.append(input, view.list, footer)`. In
  `docs/readability.txt` the JS is 9 lines shorter, 88,219 to 88,210, so
  the share of `renamed x$1`, whose count is the same, grows, 855.1 to
  855.2: no name is renamed that wasn't.
- A rest of a collection a program holds, `el.append(...children)`, isn't
  bound: a tuple's length is known. Nor was it before.
- The js crate's and node's hand-written rest parameters are still
  slices, ADR 0221's first form: `math::max(&[a, b])`, `path::join(&[..])`,
  `object::assign(target, &[..])`, whose `&[Option<&Unknown>]` is
  narrower than TypeScript's `any[]`.

## What holds it

- `test/webapi-bindings.test.ts`, "a rest parameter is a tuple, each
  argument its own type": React's `console.error` of a format and two
  values, a value of any shape, `(x,)` and `()`, `assert`'s and
  `timeLog`'s data, an element's `append` of an element and text and
  `classList.add` of a `&str` and a `String`, each run; a `None` before
  none left out and before a spread kept; and a value not a tuple
  refused, by its message.
- `test/compiler.test.ts`, the webapi forms' `app.append(input, "!")`.
- Mutations in `scripts/mutations/lower/binding_calls.ts`: a `None`
  before a rest of none given.
