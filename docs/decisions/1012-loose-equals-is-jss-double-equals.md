# 1012. `js::loose_equals` is JS's `==`

Status: Accepted. The coordinator's direction (React port gap rd78);
beside [0721](0721-strict-equals-is-jss-triple-equals.md)'s
`js::strict_equals`.

## Context

React DOM's ReactDOMInput.updateInput compares an input's text with the
value it's given by JS's loose equality, on purpose:

```js
// We explicitly want to coerce to number here if possible, so that
// other spellings of the same number (e.g. "0.0" mid-edit) aren't
// clobbered while the user types.
if (node.value != (value: any)) { node.value = toString(value); }
```

`"0.0" != 0` is `false` in JS. The js crate bound only `===`; Rust's `!=`
of `node.value()` and the value's text, `"0.0" != "0"`, is `true`, so the
port would clobber what the user is typing: a results change.

## Decision

**`js::loose_equals(a, b)`, of any two values, `impl IntoUnknown`, is
`a == b`** (`link_name = "=="`, `JsForm::LooseEquals`), its negation
`a != b`. It's the JS boundary's, for a port of JS that compares so on
purpose; any other comparison is Rust's `==`, or `js::strict_equals`'s
`===` where JS asks that. It runs what its values' `valueOf` and
`toString` run, so it isn't one that "runs nothing" (ADR 0867).

Case ([0262](0262-when-rust-and-js-disagree.md)): results change: Rust's
`==` can't say what JS's says, so the program says it, as `strict_equals`
does.

## Consequences

The guide's interop types page says when to write it. Held by
`test/js-any-values.test.ts`, "loose_equals is JS's ==, which coerces",
run in Node (`"0.0"` and `0`, `null` and `undefined`, `"1"` and `true`),
and the mutation `loose-equals-strict` (`lower/binding_calls.ts`).
