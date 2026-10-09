# 0307. A trait's binding method is read of the value itself

Status: Accepted. Extends [0049](0049-traits-and-generics.md): which bounds
take a dictionary.

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev's useSandpackLint filters ESLint's errors and CodeMirror's
diagnostics with one function, whatever has a `ruleId`:

```ts
const isReactRuleError = (error: any) => error.ruleId != null;
setLintErrors(errors.filter(isReactRuleError));
return codeMirrorErrors.filter(isReactRuleError);
```

Rust says "whatever has it" with a trait, and rust-js gives a bound a
dictionary of its impl's methods: `isReactRuleError(e,
lintErrorRuleIdentified())`. A trait method marked as a binding, the
`get ruleId` a struct's method already is, was refused: it isn't the
crate's.

## Decision

**A trait's provided method marked `#[rust_js::link_name]` is a binding,
which JS calls or reads of the value itself; a bound of a trait of only
those takes no dictionary, and an impl, empty, writes none nothing
reads.**

```rust
trait RuleIdentified {
    #[cfg_attr(rust_js, rust_js::link_name = "get ruleId")]
    fn rule_id(&self) -> &Option<String> {
        unreachable!()
    }
}
impl RuleIdentified for LintError {}
impl RuleIdentified for Diagnostic {}

fn isReactRuleError<E: RuleIdentified>(error: &E) -> bool {
    error.rule_id().is_some()
}
// function isReactRuleError(error) { return error.ruleId != null; }
// errors.filter(isReactRuleError)
```

- It's the link name a struct's method takes, `get x`, `x`, or `this`:
  one spelling for a binding, wherever it is.
- A trait of bindings is a marker trait's kind (ADR 0098): a `dyn` of it
  still carries its impl's dictionary, for its drop, and only then is
  that dictionary written. So is a marker trait's now, which was written
  whether read or not.
- Like any binding, it's unchecked: an impl's type must have what it
  reads.

## Why

- **It's the JavaScript a person writes**: the original's
  `error.ruleId`, and one function for both.
- **It's TypeScript's**: an interface of what a value has, `{ ruleId?:
  string | null }`, which any value with it meets. ReScript's open object
  types, `{.."ruleId": ..}`, say the same.
- **It's tested**: a bindings test filters two types' values through one
  generic function, run, with no dictionary written; mutations give the
  bound a dictionary, refuse the impl, and write its dictionary unread.
