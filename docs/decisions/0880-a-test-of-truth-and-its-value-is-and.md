# 0880. `a ? b : a` is `a && b`

Status: Accepted. Mirrors [0275](0275-nullable-writes-null.md)'s
`a ? a : b`, `a || b`, of the other branch.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, no observable
difference: of a test that reads the same each time it's read, `a ? b :
a` and `a && b` are each `b` where `a` is truthy and `a` where it isn't;
`&&` reads `a` once where the conditional reads it twice, which only a
getter could tell, and one a getter may answer stays a conditional.

## Context

React's shared/enqueueTask reads node's `require` off `module`:

```js
const requireString = ('require' + Math.random()).slice(0, 7);
const nodeRequire = module && module[requireString];
```

Its port had no way to write JS's `&&` of a value of any shape. `if
js::truthy(module) { js::get(module, &require_string) } else { module }`
is the Rust of it, a test of the value's truth (ADR 0271) whose `else`
is the value, as `if js::truthy(a) { a } else { b }` is `a || b`; it
printed `module ? module[requireString] : module`.

Two more of the port's questions had their answers already, said here
where it's asked:

- **`'require' + Math.random()`** is `"require".to_owned() +
  &js::concat_text(js::math::random())`, JS's `+` of a value (ADR
  0310), printed `"require" + Math.random()`; `js::shown`'s is a
  template's text (ADR 0312), `toString` first, which `+` isn't of an
  object, so it stays `` `${x}` ``.
- **`Some(js::cast(v))` of an `undefined`** is `None`: `Some` of a
  function is the function itself in JS (ADR 0051), so a cast that
  vouches for a function of what's `undefined` vouches wrongly. It's
  `unsafe`, unchecked; the cast of what may be `undefined` is to an
  `Option`, `js::cast::<Option<fn(..)>>(v)`, which `js::cast`'s
  `# Safety` and the guide now say. No diagnostic: the cast's argument is
  an `Option<&Unknown>` whatever the program knows of it.

## Decision

**`a ? b : a`, of an `a` that's one path and reads the same each time,
is `a && b`.** Preparation writes it, beside `a ? a : b`'s `a || b`.

## Why

- **It's how JS writes it**, and the Rust is the one a port writes of a
  test of truth: no `js::and` beside `if`, which would be a second way.
- **A getter's read stays a conditional**: `o.k ? o.j : o.k` reads `o.k`
  twice where it's falsy, which `&&` wouldn't.

## Alternatives

- **A `js::and(a, b)` binding**: a function of two values evaluates `b`
  first, where `&&` doesn't, and is a second way to write an `if`.
- **`Option::filter`/`and_then` of the truth**: `module.filter(|m|
  js::truthy(Some(m))).and_then(..)` reads worse than the `if`, and gives
  `None` where `a` is a falsy value, not the value.

## Consequences

- Tests: `bindings`'s truthiness test has `module && module[key]`, run
  with an object, `0` and `undefined`, and a getter's read kept a
  conditional. Mutations: the rule left out, and of a getter's read.
