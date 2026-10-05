# 0199. A component takes no drop of its type parameters

Status: Accepted. Amends [0098](0098-destructors.md) and [0190](0190-generic-no-destructor.md).

## Context

A generic function takes a drop for a type parameter its caller may give
a value with a destructor (ADR 0098), and a library's takes one for each
that isn't `Copy`, its consumers being callers it never sees (ADR 0100).
But React calls a component, `Card(props, secondArg)`, with a value of its
own after its props, never a drop, as JSX, a `jsx!`'s or TypeScript's,
gives it none: react.dev's `Button`, its children a type parameter, built
by Cargo as a library, took one, `Button(param, dropC)`, flagged what it
held to drop it, and so didn't take its props apart.

## Decision

**A component, as `jsx!` takes one, a capitalized function of its props
that returns an `Element`, takes no drop of its type parameters, and one
given a type with a destructor is an error:**

```text
error: rust-js does not support giving component `Holder`'s `T` a type with a destructor yet: React gives a component no drop
```

- **Its body is as of a type with none**: `function Card({ title, children })`,
  its children moved with no flag.

## Why

- **It's exact**: what React can't give, the component isn't given, and a
  type it would need one for is refused, where it was given a drop React
  passes something else as. A JSX test builds a library's generic
  component with no drop, and refuses one given a type with a destructor.

## Costs

- **A component's type parameter can't be given one with a destructor.**
