# 0654. A dictionary's member of any value is a `dyn IntoUnknown`

Status: Accepted. Amends [0024](0024-web-crate.md)'s webapi generator,
whose dictionary left a member of `any` out.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: the value is given
to JS as it is.

## Context

React's `reportGlobalError` makes the event an uncaught error would:

```js
const event = new window.ErrorEvent('error', {
  bubbles: true,
  cancelable: true,
  message: message,
  error: error,
});
```

TypeScript's `ErrorEventInit` has `error?: any`; webapi's had no `error`.
The generator left out each dictionary member of `any`, "as a struct
field can't be generic": a parameter of `any` is a generic, and a field
can't be one. So `CustomEventInit` had no `detail`, `MessageEventInit` no
`data`, and `PromiseRejectionEventInit` neither of its members, `promise`,
WebIDL's `object`, TypeScript's `Promise<any>`, required, and `reason`.

## Decision

**A dictionary's member of any value is a `&'a dyn IntoUnknown`, as a
rest parameter's item of `any` is (ADR 0640): `Option<&'a dyn
IntoUnknown>` where it's optional.** Given as it is:

```rust
error_event::new("error", Some(ErrorEventInit {
    bubbles: Some(true),
    cancelable: Some(true),
    message: Some(&message),
    error: Some(&error),
    ..Default::default()
}))
```

```js
new ErrorEvent("error", { bubbles: true, cancelable: true, message, error });
```

- **19 members are bound**: 18 of `any`, `ErrorEventInit.error`,
  `CustomEventInit.detail`, `MessageEventInit.data`, `NotificationOptions.data`,
  `PromiseRejectionEventInit.reason`, `PopStateEventInit.state`,
  `PerformanceMarkOptions.detail`, `PerformanceMeasureOptions.detail`,
  `RequestInit.window`, `MemoryDescriptor.initial` and `maximum`, and the
  navigation API's `info` and `state`, seven; and
  `PromiseRejectionEventInit.promise`, by `FIELD_TYPES`, TypeScript's
  `Promise<any>`. Three dictionaries had no member Rust could take and are
  made now, `NavigationOptions`, `NavigationReloadOptions` and
  `NavigationUpdateCurrentEntryOptions`, so `navigation.updateCurrentEntry(options)`
  is bound too.
- **A binding crate's `dyn` is the value it's of**: `dyn IntoUnknown`,
  of a trait rust-js doesn't compile and that isn't std's, was refused as
  a value where a struct of one was made, `..Default::default()`'s.
- **`PromiseRejectionEventInit` has no `Default`**: its `promise` is
  required, as TypeScript's is, and so is its constructor's dictionary.
- A struct that borrows now, `CustomEventInit<'a>`, is named with its
  lifetime.

## Why

- **TypeScript's members, each**: a dictionary without `error` can't make
  the event React makes.
- **One form of "any value Rust gives"**: a rest item's, `dyn
  IntoUnknown`, which `Some(&value)` coerces to, whatever its type; a
  struct, a `&Unknown`, an `Option<&Unknown>` of one JS gave.

## Alternatives

- **`Option<&'a Unknown>`**: a value JS gave would go as it is, `error`,
  but a Rust value, `Some(&point)`, would need a cast.
- **A type parameter of the dictionary, `ErrorEventInit<'a, E>`**: a
  second type to name of each, and a `None` of no type to infer.

## Consequences

- **A member of `object` is still left out**, 6 of them, a `dyn Any`'s:
  only a binding's argument is given to JS as its value (ADR 0331), not a
  struct's field, which would be its pair. And a list of `any`s.
- **`promise` takes any value**, where TypeScript takes a promise.

## What holds it

- `test/bindings.test.ts`, "a dictionary's member of any value is a dyn
  IntoUnknown": React's `ErrorEvent`, with and without an error, a Rust
  struct as a `CustomEvent`'s detail, and a `PromiseRejectionEvent`'s
  required `promise`, each as JS and run.
- `test/webapi-coverage.test.ts`: the crate is its generator's, and
  `Navigation.updateCurrentEntry` bound.
- Mutation `binding-dyn-refused`.
