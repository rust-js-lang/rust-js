# 0776. `js::get`, `js::has`, `js::set` and `js::delete` take any value

Status: Accepted. Amends [0225](0225-unknown-values.md), whose `get`, `has`
and `set` took a present value, `&Unknown`, and [0697](0697-delete-is-jss-delete.md),
whose `delete` did. Follows [0721](0721-strict-equals-is-jss-triple-equals.md)'s
`strict_equals`, whose operands are any value, `impl IntoUnknown`.
Amended below (#462): a JS operator of any value is what JS may answer, as
a getter's read, so never dropped unread; and with it, ADR 0660's
`Expr::caught` calls a closure of one value, which applied arrows make its
body.

Case: C ([0262](0262-when-rust-and-js-disagree.md)): results changed. A
port had to guard what JS throws of, so a program that relies on the
throw, as React's tests do of its message, behaved otherwise.

## Context

React's `lazy` writes `'default' in moduleObject` and `moduleObject.default`
of what a loader resolved to, which may be `undefined`; `forwardRef` and
`memo` write `render.displayName = name` of a `render` that may be
`undefined`. In JS each throws a `TypeError` of `undefined` or `null`,
`Cannot use 'in' operator to search for 'default' in undefined`, and React's
tests check that message. `js::get`, `js::has` and `js::set` took a
`&Unknown`, a value that's never `undefined` nor `null`, so the port wrote
`module_object.is_some() &&` and `module_object.and_then(..)`, which
answer where JS throws: a behaviour difference, and Rust bent to get there.

TypeScript reads a property of an `any`, which `undefined` may be.

## Decision

**`js::get(value, key)`, `js::has(value, key)`, `js::set(value, key, to)`
and `js::delete(value, key)` take any value, `value: impl IntoUnknown`, as
`js::strict_equals` does: an `&Unknown`, an `Option<&Unknown>`, a
`&JsObject`, any value JS has. Each is JS's plain operator, `value.key`,
`"key" in value`, `value.key = to`, `delete value.key`, so of `undefined`
or `null`, a `None`, it throws JS's `TypeError`, a panic `catch_unwind`
catches (ADR 0710).** The JS of an existing caller is what it was.

```rust
pub fn default_of(module_object: Option<&Unknown>) -> Option<&'static Unknown> {
    js::get(module_object, "default")
}

pub fn has_default(module_object: Option<&Unknown>) -> bool {
    js::has(module_object, "default")
}

pub fn name(render: Option<&Unknown>, name: &str) {
    js::set(render, "displayName", name);
}
```

```js
export function defaultOf(moduleObject) {
  return moduleObject.default;
}

export function hasDefault(moduleObject) {
  return "default" in moduleObject;
}

export function name(render, name) {
  render.displayName = name;
}
```

## Why

- **It's JS's meaning**, which a binding of JS's own operator asks for:
  of `undefined`, the `TypeError` JS throws, its message JS's.
- **One signature for an operator of any value**: `strict_equals`,
  `get`, `has`, `set` and `delete` each take `impl IntoUnknown`, what
  TypeScript's `any` is given.
- **Nothing is bent**: `js::has(module_object, "default")` is upstream's
  `'default' in moduleObject`, with no guard upstream hasn't.

## Rejected

- **An `Option<&Unknown>` parameter**: a `&JsObject` or a `Some(&value)`
  would need converting, `Some(js::unknown(..))`, where `impl
  IntoUnknown` takes each as it is.
- **A second binding of each, of an `Option`**: two ways to read one
  property.

## Consequences

- A `None` given is no longer refused by rustc; it's JS's throw. A port
  that means "a property, if there's a value" writes Rust's
  `value.and_then(|v| js::get(v, key))`, `value?.key`.
- Held: `test/js-object.test.ts`, "get, has and set take any value, and
  throw of nothing as JS does", runs each of `undefined`, `null` and an
  object, and a `catch_unwind` of the throw; every other caller's JS is
  as it was (the lowering, JSX, docs and snapshot tests). No compiler rule
  changed, only the js crate's signatures, which rustc checks, so there's
  no mutation of one: the test fails to compile without them.

## Amendment (2026-11-02): a JS operator of any value is never dropped unread (#462, P0)

`js::has(m, "default")` throws of an `m` that's `undefined`, but nothing
said so to the passes that drop what nothing reads: `"default" in m` was
an operator of two variables, which `has_effects` took to do nothing and
`reads_only_vars` to read only them. So

```rust
pub fn caught(m: Option<&'static Unknown>) -> bool {
    std::panic::catch_unwind(|| js::has(m, "default")).is_err()
}
```

was `try { return false; } catch { return true; }` once tidy's applied
arrows made `(() => "default" in m)()` its body and the value nobody read
went (`js::value_of`), so `caught(undefined)` was `false` where Rust's is
`true`; and `let _present = js::has(m, "default");` was no statement at
all (prepare's `unread`), so nothing threw where Rust panics.
`js::loose_equals`'s `==`, which runs an object's `valueOf`,
`js::instance_of`'s `instanceof`, which throws of what's no function, and
`js::to_number(m).unwrap()`'s `+m`, which throws of a symbol, went alike.

**A binding's JS operator of any value is what JS may answer, as a
getter's read is (`Known::Getter`)**: `in`, `==`, a given constructor's
`instanceof`, and a conversion the js crate types a `Result`, `"" +
value`, `+value`, `value - n` (`binding_calls.rs`). So it has effects and
may throw, reads more than variables, and is kept where nothing reads it,
`try { "default" in m; return false; }`, as a getter's `m.value;` was;
what's made again of it keeps that, `m != 1` of `!(m == 1)` and a unary
tidy remakes (`Facts::of_evaluation`). Of text, `js::to_number(s)` throws
nothing and runs nothing, so an unread one is still no statement. A
class's own `x instanceof Error` is unchanged: of any value it throws
nothing.

With that, `Expr::caught` no longer takes a closure of one `return v;`
apart itself: its `try` is the closure called, `(() => v)()`, which tidy's
applied arrows make `v` with the closure's place, so
`catch_unwind(|| inner(n))`'s `return inner(n);` maps to the closure's
`inner(n)`, not to the `Ok` arm's line, as the value it took apart had no
place. Every corpus case's JS is the same either way (ADR 0660's
`Expr::caught`).

Case ([0262](0262-when-rust-and-js-disagree.md)): C. The result changed:
JS's throw is Rust's panic, which the program asked for.

Held by `test/lowering-unwind-arms.test.ts`, "a caught value nothing reads
that may throw is still evaluated", run in Node: `in` of `undefined`,
`null`, text and objects, a getter that throws, `==` and `!=` of an object
whose `valueOf` throws, `instanceof` of what's no function, `+` of a
symbol and a `BigInt`, an unread `let` of `in` outside any `try`, and `+s`
of text, which stays dropped; and "a try's call of a closure of one value
maps to the closure's body". Corpus case `catch_unwind_unread_value`
against native Rust: a closure's value nothing reads that may panic, a
call, an index, a division, an `unwrap`, a `RefCell` and a `str` cut.
Mutations `in-answers-nothing`, `loose-equals-answers-nothing`,
`instance-of-given-answers-nothing`, `conversion-of-any-answers-nothing`,
`conversion-of-text-answers` (`lower/binding_calls.rs`),
`reads-only-vars-of-js-operators` (`js/expr_walk.rs`),
`evaluation-facts-forgotten` (`js.rs`), `negations-evaluation-lost`
(`lower/mir/tidy.rs`) and `negated-loose-evaluation-lost` (`js/make.rs`).
The mutation `caught-arrow-called` went with the arm it mutated: calling
the closure is now the one way, and the map test fails with the arm.
