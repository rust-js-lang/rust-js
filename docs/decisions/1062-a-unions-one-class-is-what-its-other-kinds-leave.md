# 1062. A union's one class is what its other kinds leave

Status: Accepted (rd506's audit, P1, 2026-11-02; the coordinator's
decision on the user's behalf). Amends
[0214](0214-untagged-enums.md) (an untagged enum's variant is told
apart by its value's kind).

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior is kept. A test of `instanceof File` where the host has
no `File` global throws a `ReferenceError`, where Rust's `match` runs on.

## Context

A `match` of an untagged enum tests its arms in the order the program
wrote them, the last what's left. A form's entry, webapi's
`FormDataEntryValue`, is a `File` or a string:

```rust
match data.get(name) {
    Some(FormDataEntryValue::File(_)) => 1,
    Some(FormDataEntryValue::Str(_)) => 2,
    None => 0,
}
```

printed `if (get instanceof File) { return 1; } return 2;`. A host
without the `File` global, Node 18, a worker of an older engine or a
test's environment, throws `ReferenceError: File is not defined` there,
though a `typeof get === "string"` tells the two apart with no global.
React's Flight reply client met it of getReader's union of readers,
`reader instanceof ReadableStreamDefaultReader`, in Jest's environment,
which has `ReadableStream` but not its readers (rd506, P1).

## Decision

**Where an untagged enum's one variant is a class and each other is a
kind JS tells by the value alone, the class's branch is the last, what's
left, and each other's is tested**: `typeof`, `Array.isArray`, a tuple's
length, a literal, `undefined` and `null`
(`Recognition::lone_class_variant`, consumed by `mir/structure.rs`'s
switch).

```js
if (get !== null) {
  if (typeof get === "string") {
    return 2;
  }
  return 1;
}
return 0;
```

- **Two classes or more keep `instanceof`**: one of them has to be
  tested by its class. webapi's unions of them are the DOM's, IDB's,
  CSSOM's and canvas's (`RenderingContext`, `HTMLCollectionOrElement`,
  an `IDBRequest`'s source), whose classes a host has wherever it has
  the API that gives them.
- **An object, a test function's or a member's kind keeps its test**:
  an object's leaves a class out by `instanceof` too.
- **The arms the others take stay in the order the program wrote them**;
  only the class's moves to the end, where it's what's left.

## Alternatives

- **`typeof File !== "undefined" && get instanceof File`**: a guard
  upstream never writes, before every class test.
- **The program's order, the class first**: the `ReferenceError`.

## Consequences

- A `match` whose first arm is the class reads its arms in another order
  in JS: the class's last.
- Held by `test/webapi-objects.test.ts` ("a union of one class is told
  apart by the others' tests, no instanceof of a global"), run with
  `File` deleted from the global object, and the mutations
  `lone-class-tested` (`untagged.rs`) and `lone-class-branch-dropped`
  (`mir/structure.rs`).
