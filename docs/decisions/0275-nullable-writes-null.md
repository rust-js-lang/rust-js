# 0275. A nullable field's `None` is `null`

Status: Accepted. Extends [0196](0196-typescript-declarations.md). Amended by
[0772](0772-a-nullable-cell-field-writes-null.md): a nullable `Cell`'s writes. Extended by
[0810](0810-an-optional-nullable-field-is-absent-until-set.md): an optional one is left out where it's made `None`. Extended by
[0861](0861-a-functions-own-nullable-parameter-is-set-null.md): a parameter of the crate's own function, set `None`.

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev's errors page gives Next.js its props, a code or `null`:

```tsx
return {props: {content, toc, meta, errorCode: code, errorMessage: code ? errorCodes[code] : null}};
```

Next.js sends them as JSON, which has no `undefined`: it refuses a prop
that is. A `#[rust_js::nullable]` field took `null` (ADR 0196), but its
`None` was `undefined`, as every `None` is (ADR 0030).

## Decision

**A `#[rust_js::nullable]` field is TypeScript's `T | null` when it's
made: its `None` is `null`.**

- `None` is `null`, and a `Some(..)` or a constant is its value.
- An `Option` that may be `None` is `value ?? null`.
- One chosen by a `match` or an `if` whose arms are each `Some(..)` or
  `None` is its conditional with `null` for each `None`: `code ? errorCodes[code]
  : null`, as the errors page writes it, and `code || null` where it gives
  what it tests (`a ? a : b` is `a || b` wherever it's printed).
- Another such field's value, read or bound by a pattern and not set again,
  is `null` or a value already, as it is: `{ code, message }` of one
  destructured.

```rust
Props { code, message: None, title: None }
```

```js
return { code: code ?? null, message: null, title: undefined };
```

Its declaration stays `?: T | null`: a TypeScript caller may leave it out.

## Why

- **No program can tell**: rust-js reads `null` and `undefined` alike as
  `None` (ADR 0030); only JS that reads the object, JSON's say, sees which.
- **It's what the TypeScript type says**, `string | null`, and what the
  page wrote.
- **It's tested**: a compiler test makes one of `None`, of an `Option`, of
  a `Some`, and of another's field, and its JSON keeps each; mutations make
  each `undefined` or `?? null`.

## Since

- **A binding's parameter is nullable too, `#[rust_js::nullable(replacer)]`
  on its function**, naming it: its `None` is `null`, an `Option` that may
  be `None` is `value ?? null`, as a field's is. On the function, as a
  parameter's own attributes are dropped in an `extern` block and aren't in
  a crate's metadata. A name that isn't a parameter, or one that isn't an
  `Option`, is an error. `json::stringify_with(&value, None, 2)` is
  `JSON.stringify(value, null, 2)`, as react.dev's Sandpack template
  writes it. Case A: rust-js reads `null` and `undefined` alike as `None`,
  and `JSON.stringify` takes either as no replacer.

## Amendment: a copy of one

A nullable field given a copy of another's value, `error.rule_id.clone()`,
is given it as it is, as one given the field itself is: it's `null` where
it's `None` already, react.dev's runESLint's `ruleId: error.ruleId`.

## Amendment: a `Some(..)` given to a nullable parameter

A binding's nullable parameter given a `Some(..)` wrote it `value ??
null`, `Object.create(Object.prototype ?? null)` ([0666](0666-the-js-crate-binds-objects-reflection.md)),
where a field given one writes it as it is: whether an `Option` is
`null` or a value already had two answers, the field's (`nulled`, in
`mir/rvalues.rs`) and the parameter's, which knew only a literal, or a
function, an object or an array made in the call (`Expr::is_made_here`).
A parameter asks the field's question now, of each operand its call is
given: a `Some(..)`, a nullable field read, or a variable bound to one,
is as it is, `Object.create(Object.prototype)`, `JSON.stringify(items,
keys, 2)` of `Some(keys)`. A `Some` of what's made in the call is one of
them, so `is_made_here` is gone. Case A, as before: the value is the
`Some`'s, which is never `undefined`.

Held by `test/lowering-options.test.ts`, "a binding's nullable
parameter's None is null", `test/js-object.test.ts`'s `Object.create`,
and the mutations `nullable-param-some-coalesced` (which takes in
`mir-nullable-made-coalesced`) and `nullable-param-nulled-unasked`.

## Amendment: a nullable attribute's setter takes a value or `None` (2026-10-28, the user's decision)

React's ReactInputSelection keeps an input's selection as it reads it,
`{start: input.selectionStart, end: input.selectionEnd}`, `number | null`
each, and writes it back, `input.selectionStart = start`. The webapi
crate's getter was an `Option<f64>`, but its setter took `impl
Into<f64>`, so a `null` start couldn't be written back, and
`node.textContent = null`, which clears a node, couldn't be written at
all (React port gap rd46).

**The user decided: an attribute TypeScript types `T | null` is set of
`impl Into<Option<T>>`, one API taking a value or `None`**, its parameter
`#[rust_js::nullable(value)]`, so `None` is `null`:
`node.set_text_content("x")` is `node.textContent = "x"`,
`node.set_text_content(None)` `node.textContent = null`, and
`input.set_selection_start(start)` of a nullable field's `start`
`input.selectionStart = start`, as upstream writes it. A reference it
takes is of a lifetime the setter names, `set_text_content<'a>(&self,
value: impl Into<Option<&'a str>>)`, as `impl Trait` elides none. Not a
union's, which its enum's `impl Into..` takes, nor a sequence's,
`&[&Element]`, which an array literal reaches by a coercion `Into`
wouldn't make. `Math.min` keeps TypeScript's `f64`s: the port writes
`end.unwrap_or(0.0)`, `end ?? 0`, the value JS's `Math.min(null, n)`
coerces `null` to.

Its cost: `&String` is no `Into<Option<&str>>`, so text made here is
given by its `as_str()`, `output.set_text_content(n.to_string().as_str())`,
where `&n.to_string()` was; the examples write that. Case
([0262](0262-when-rust-and-js-disagree.md)): None, of what the setter
gives JS.

Held by `test/webapi-bindings.test.ts`, "a nullable attribute's setter
takes a value or None", run in Node.

## Amendment: a function's parameter that takes `null` is an `Option` (rd151)

webapi's generator made a setter of `T | null` take `impl
Into<Option<T>>`, but a function's required parameter WebIDL types
`Node? child`, TypeScript's `Node | null`, took `&Node`: narrower than
TypeScript, so React's ReactFiberConfigDOM's
`parentInstance.moveBefore(child, null)` had no Rust. **A required
parameter of a nullable type is an `Option` of it, its `None` `null`**
(`#[rust_js::nullable(child)]`, the generator's `signatures`): `insert_before
(node, child: Option<&Node>)`, `move_before`, `contains`,
`is_same_node`, `is_equal_node`, `lookup_prefix`, `set_attribute_ns(namespace:
Option<&str>, ..)` and the others of the namespace, 151 in all. Not a
union's, nor `any`'s, which take `null` already. Callers write
`Some(&next)` and `None`, `create_element_ns_named(Some(SVG_NAMESPACE), ..)`.

Case N: the binding's type is TypeScript's; each JS call is what it was,
or the `null` it couldn't pass.

Held by `test/webapi-bindings.test.ts`, "a parameter that takes null is
an Option", run; "the webapi crate is what its generator writes".

