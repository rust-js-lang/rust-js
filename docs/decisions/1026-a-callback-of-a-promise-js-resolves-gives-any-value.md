# 1026. A callback of a promise JS resolves gives any value, and a string in an array is a `String`

Status: Accepted (rd206, a port gap; P1). Extends [0260](0260-webapi-callbacks.md)'s
callbacks, [0869](0869-a-web-callback-called-once-is-fn-once.md)'s `FnOnce`,
[1021](1021-a-listener-is-a-closure-or-one-js-gave.md)'s `Copy` closure
JS keeps where Rust lent it and [0219](0219-webapi-sequences.md)'s slices.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. JS resolves what
the function gives, as WebIDL converts a callback's result to its promise
type; the function is called as before, and the array is the one the
program holds, which the browser copies as it reads it.

## Context

React DOM's startViewTransition gives

```js
ownerDocument.startViewTransition({
  update: () => {
    ...
    if (blockingPromises.length > 0) { ...; return allReady.then(afterMutationCallback, afterMutationCallback); }
    layoutCallback();
    if (pendingNavigation) { return pendingNavigation.finished.then(afterMutationCallback, afterMutationCallback); }
    else { afterMutationCallback(); }
  },
  types: transitionTypes,
});
```

and startGestureTransition `{update: mutationCallback, types:
transitionTypes}`. WebIDL types the callback `Promise<any> ()`, which
TypeScript writes `(): any`: JS resolves whatever it gives. webapi took a
`Box<dyn FnMut() -> Promise<Option<&Unknown>>>`, so the third path had to
give `Promise.resolve()` (ticks the browser waits for), `mutationCallback`
couldn't be given as it is, and `types` was a `&[&str]` the program's
`Vec<String>` had to be copied into, `transitionTypes?.slice()`, where
TypeScript's is `string[] | null`.

And the options object is lent to `start_view_transition`, a union's
`Option<&dyn ..>` (ADR 0443), so the `Box` in it was Rust's still: Rust
dropped what the closure held after the call (ADR 0873), while the browser
went on to call it, reading what was dropped.

## Decision

**A callback whose WebIDL result is a promise takes a function or closure
of any result, `Into<Callback>`: a promise, or any value, `()` and a
`None` `undefined`, as a promise's `then` takes a function of any result
(ADR 0782).** Being a trait, it gives rustc no closure signature, so a
closure's parameters are annotated, `|lock: Option<&Lock>| ..`.

- **Moved, a `Box`**: `add_handler(Box::new(..))`,
  `NavigationInterceptOptions { handler: Some(Box::new(move || ..)) }`,
  `locks.request(name, Box::new(..))`, as every callback a function is
  given (ADR 0260): JS's, which may own what it holds. Called once by
  each operation that takes it, `LockManager.request`, it's `FnOnce`
  (ADR 0869); `FnMut` otherwise.
- **Lent, `Copy` and `Fn`**: a callback a union holds, or a dictionary
  in one, `start_view_transition(Some(&..))`'s, is borrowed, so it's a
  `&dyn` of a closure that owns nothing to drop, as ADR 1021's event
  handler is: `update: Some(&update)`, `update: Some(&mutation_callback)`
  of a `fn()`, and the union takes the function itself,
  `start_view_transition(Some(&move || ..))`.
- **A string in an array a function takes is a `String`**, as one a
  function gives is: `types: Option<&[String]>`, so the program's
  `Vec<String>`, or what a getter gave, `&'static [String]`, is given as
  it is; a literal, `&["class".into()]`.
- **A union's variant of such a callback is named for it**:
  `ViewTransitionUpdateCallbackOrStartViewTransitionOptions`, declared
  `ViewTransitionUpdateCallback | StartViewTransitionOptions`, where it was
  `ListenerOrStartViewTransitionOptions`, declared with `EventListener`.

```rust
let update = move || {
    mutation_callback();
    if document.hidden() {
        let ready: Promise<()> = document.fonts().ready().then(move |_| mutation_callback());
        return Some(ready);
    }
    None
};
document.start_view_transition(Some(&StartViewTransitionOptions { update: Some(&update), types }));
```

```js
function update() {
  mutationCallback();
  if (document.hidden) {
    const ready = document.fonts.ready.then(() => {
      mutationCallback();
    });
    return ready;
  }
}
document.startViewTransition({ update, types });
```

## Why

- **It's what JS takes.** The browser resolves the result, so a function
  of any result is the callback; one fixed to a promise made the program
  write ticks upstream doesn't have.
- **`Copy` where it's lent.** The Rust that lends a value drops it when
  the borrow ends; a closure JS calls after must hold nothing that drop
  changes. That's ADR 1021's rule for a handler, and a `Box` there was
  the one place webapi let a lent closure own what it held.
- **`Box` where it's moved**, so a closure may own its state, as every
  other callback may.
- **`String`s are what arrays of strings are in webapi already**: a getter
  gives `&'static [String]`, `Vec<String>`; a `&[&str]` parameter took
  none of them without a copy.

## Consequences

- `IntoViewTransitionUpdateCallback`, `IntoNavigationInterceptHandler`,
  `IntoNavigationPrecommitHandler` and `IntoLockGrantedCallback`, each
  sealed by a trait of its own, as the union of one is.
- A closure given to one says its parameters' types.
- Each string array a function or a dictionary takes is a `&[String]`,
  `Headers`' pairs `&[&[String]]`.
- Mutations: none; the webapi crate's types are what rustc checks
  (ADR 0093). `webapi-bindings.test.ts`'s "a callback of a promise JS
  resolves is a function of any result" runs each form.
