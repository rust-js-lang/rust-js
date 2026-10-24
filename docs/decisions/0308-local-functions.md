# 0308. A function written in a body is that body's

Status: Accepted; amended by [0696](0696-a-constructor-is-a-fn-item-where-its-written.md):
in the block it's written in, after its comments.
Amends [0296](0296-named-function-expressions.md): where a function a
block makes and doesn't give is.

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

## Since

- **Its body is prepared as the body it's in**: each pass over a module's
  lists of statements, `s += t` and no `else` after a branch that leaves,
  reaches it, as it reaches a closure's (`js::each_block_mut`).
  (Amended: they passed it over, `s = s + t` left so; `nested_fn_prepared`
  runs it against native Rust.)

## Amendment: not of a trait's default

A function written in a trait's default method, base64's `Engine::encode`'s
`inner`, was declared in the default's body; but each impl's dictionary
copies that body into its own module (ADR 0185), where what `inner` calls
wasn't imported, and the copy called an `inner` its trait's module never
had. One written in a trait's default is the trait module's, which each
copy imports. `test/corpus/default_inner_fn.rs` holds it, of an impl in
another module.

## Amendment: in the block it's written in

A function written in an `if`'s block was declared in its body's own
list, after the `if`, as no statement of that list was written after it;
and the comment above it was lost. It's declared in the block it's
written in, before what's written after it there, with
its comments (ADR 0696).
