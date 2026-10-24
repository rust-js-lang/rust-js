# 0735. A comment in a macro, before the `fn` it writes, is the function's

Status: Accepted. Amends [0621](0621-comments-around-macros-and-at-a-modules-end.md),
whose comment before a macro's opening line is its code's.

Case ([0262](0262-when-rust-and-js-disagree.md)): it can't be observed:
a comment is no code.

## Context

React's binaryToComparableString says why its parameter is narrower than
upstream's, inside the macro that exports it:

```rust
// Turns a TypedArray or ArrayBuffer into a string that can be used for comparison
// in a Map to see if the bytes are the same.
js::export_default! {
    // GAP: upstream takes any `$ArrayBufferView` ..
    fn binary_to_comparable_string(view: &Uint8Array) -> String {
```

The first comment was the function's (ADR 0621); the second was dropped.
A function a macro writes has the macro's span, so it starts where
`js::export_default!` does: the comment inside comes after its start,
and the next start, its body's first statement, has the signature before
it, code.

## Decision

**A function a macro writes, whose name the program wrote in it, starts
on the line its name is on** (`mir::queries::written_from`): a comment
before it inside the macro is before it, and one before the macro's
opening line still is too, the opening between them as ADR 0621 has it.

## Consequences

- The port's `// GAP:` comment is above `export default function`, after
  the one above the macro.
