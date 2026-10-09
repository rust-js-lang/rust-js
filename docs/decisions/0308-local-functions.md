# 0308. A function written in a body is that body's

Status: Accepted. Amends [0296](0296-named-function-expressions.md): where
a function a block makes and doesn't give is.

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev's useSandpackLint makes `isReactRuleError` in the callback that
uses it:

```ts
const onLint = linter(async (props) => {
  ..
  const isReactRuleError = (error: any) => error.ruleId != null;
  setLintErrors(errors.filter(isReactRuleError));
  return codeMirrorErrors.filter(isReactRuleError);
});
```

Rust writes a function there as an item, `fn isReactRuleError(..)`, and
rust-js wrote every item at its module's top: the function left the
callback the original keeps it in.

## Decision

**A function written in a function's body, that only it and others
written so name, is a function declaration where it's written there.**

```rust
let onLint = linter(async move |props: &EditorView| {
    ..
    fn isReactRuleError<E: RuleIdentified>(error: &E) -> bool {
        error.rule_id().is_some()
    }
    setLintErrors.set(errors.into_iter().filter(isReactRuleError).collect());
    codeMirrorErrors.into_iter().filter(isReactRuleError).collect()
});
```

```js
const onLint = linter(async (props) => {
  ..
  function isReactRuleError(error) {
    return error.ruleId != null;
  }
  setLintErrors(errors.filter(isReactRuleError));
  return codeMirrorErrors.filter(isReactRuleError);
});
```

- A declaration, not `const f = () => ..`: its block may call it before
  it, as Rust's may, and JS hoists a declaration in its block. One
  written after a destructor's `let` is before that `let`'s `try`, so
  what's before can call it.
- One a static's initializer calls stays the module's: that initializer
  is the module's (ADR 0037).
- It's made readable as a module's function is.

## Why

- **It's the JavaScript a person writes**: the original's function, in
  the callback it's in.
- **It's the same program**: a Rust function captures nothing, and its
  name is its block's alone, so it's the same function wherever it's
  made.
- **It's tested**: a lowering test runs functions written in a body and
  in a closure, one called before it's written, one after a destructor's
  `let`, one calling another module's, and one a static calls; mutations
  write them at the module's top, at the body's end, in the `try`,
  unlinked, unprepared, and the static's in its body.
