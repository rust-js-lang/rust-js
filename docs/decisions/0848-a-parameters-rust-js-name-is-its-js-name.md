# 0848. A parameter's `#[rust_js::name]` is its JS name

Status: Accepted. Extends [0573](0573-every-rust-name-is-camel-case-in-js.md),
whose `#[rust_js::name]`, the one way to name an item in JS, now names a
parameter too, and [0678](0678-a-parameters-underscore-isnt-its-js-name.md),
whose `lower::parameter_name` reads it.

Case ([0262](0262-when-rust-and-js-disagree.md)): A. A parameter's name
changes nothing a program does; `names.rs` still names it apart where it
would meet another binding.

## Context

React's `useContext` takes the context by a capitalized name, as a
component's is:

```js
export function useContext(Context) {
  const dispatcher = resolveDispatcher();
  return dispatcher.useContext(Context);
}
```

Rust names a parameter in snake_case, `non_snake_case` warns of `Context`,
and an `#[allow]` for rust-js's sake is no port's (AGENTS.md). The port
wrote `#[cfg_attr(rust_js, rust_js::name = "Context")] context:
&ReactContext<T>`, which rust-js ignored, silently: the JS was
`useContext(context)`, and its `.d.ts` too.

## Decision

**A parameter's `#[rust_js::name = ".."]` is its JS name**: its body's,
a closure's that it's of, and its declaration's, `useContext(Context:
ReactContext<T>)`. `lower::parameter_name`, the one place a parameter's JS
name is made, asks `bindings::given_parameter_name` first, read from the
function's body, as `#[rust_js::rest]` is (ADR 0664): rustc keeps a
parameter's attributes nowhere else. Without one, the name is ADR 0678's.

It's refused, saying what to write, where it names nothing it can
(`validation::reject_parameter_name_misuse`):

- of a pattern, `(x, y): (u32, u32)`, or `self`, which has a name of its
  own: "names a parameter that's a name, `context: T`";
- not a string, `#[rust_js::name]`;
- not a name a JS variable may have, `"my-name"`, or a word JS keeps;
- the name it has anyway, `#[rust_js::name = "value"] value`: a second way
  to write it, as ADR 0573 refuses of an item's.

## Why

- **One way per job**: `#[rust_js::name]` already says an item's JS name
  where it isn't the camelCase one. A parameter's is the same job; a
  second attribute, or `#[allow(non_snake_case)]`, would be another way.
- **Silently ignored is a lie the compiler tells**: the program says a
  name the JS doesn't have. Making it work, rather than refusing it, keeps
  upstream's name, which the port compares line by line.

## Alternatives

- **Refuse it, and write `context`**: the JS's parameter differs from
  upstream's, a difference AGENTS.md asks rust-js to fix.
- **Read a parameter's Rust name as written, `Context`**: rustc warns, and
  every other parameter would have to be checked for case.

## What holds it

`test/bindings.test.ts`, "a parameter's rust_js::name is its JS name", run
in Node, of a function's and a closure's parameter, its `.d.ts`, and each
refusal; mutations `parameter-name-ignored` (`lower.ts`),
`parameter-name-repeat-allowed` and `parameter-name-any-text`
(`lower/analysis/validation.ts`).
