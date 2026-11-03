# 1081. A function reads the `this` its JS caller gave it by `js::this()`

Status: Accepted. The user's decision (2026-11-03, rd683): a function's own `this`, beside `js::arguments()`; its rules here. Extends
[1048](1048-a-function-that-reads-arguments-is-given-each.md)'s
`js::arguments()`, the same rules for a function's own `this`; beside
[0972](0972-a-function-that-reads-its-this-is-with-thiss.md)'s
`function::with_this`, a closure's. Amends
[1018](1018-a-function-called-through-its-module-is-read-through-its-namespace.md):
a crate's free function may now read `this`, so one that does is called
through its module's namespace with none.

Case ([0262](0262-when-rust-and-js-disagree.md)): none for a JS caller,
whose `this` is what the function reads. Results change for a Rust
caller, whose call gives no `this`, so preserve Rust's: each call rust-js
writes gives none, `(0, M.f)()` through a namespace, and no form that
would give one, a value JS calls on what holds it, a closure or a
forwarder that's the function itself, is written.

## Context

React DOM's root server entries load their implementation where they're
called, so a test's `jest.resetModules()` gives each call the host
configs it set up:

```js
export function renderToString() {
  return require('./src/server/ReactDOMLegacyServerNode').renderToString.apply(
    this,
    arguments,
  );
}
```

`js::arguments()` is JS's `arguments` (ADR 1048), but the port had no
`this` for a function of the crate's: `function::with_this` gives a
closure its own, and a class's method's `this` is its `self`. The port
wrote `function::apply(f, None, Some(js::arguments()))`,
`.apply(undefined, arguments)` (gap rd683).

## Decision

**`js::this()` is JS's `this`, `Option<&'static Unknown>`: the `this` the
function it's read in was called with, `None` of `undefined` or `null`**,
the type `function::with_this` gives its closure's `this` (ADR 0972), so
`this.map(|this| this as &dyn IntoUnknown)` gives it on as it does:

```rust
pub fn render_to_string() -> Option<&'static Unknown> {
    function::apply(
        react_dom_legacy_server_node::render_to_string(),
        js::this().map(|this| this as &dyn IntoUnknown),
        Some(js::arguments()),
    )
}
```

```js
export function renderToString() {
  return require("./src/server/ReactDOMLegacyServerNode").renderToString.apply(this, arguments);
}
```

- **One owner**: `analysis/arguments.rs`, which finds what reads
  `js::arguments()`, finds what reads `js::this()` (`ReadsArguments::this`);
  calls ask `FnCx::reads_this`, closures and forwarders
  `reads_its_call`, a library's consumers its contract's `this`
  (`ItemFact::This`).
- **Each call rust-js writes gives none**, as Rust's does: by its name,
  `onSomething()`; through its module's namespace, `(0,
  Inner.onSomething)()`, ADR 0972's one form of a call with none, not
  `Inner.onSomething()`, which would give it the namespace; a library's
  consumer's too.
- **It's only called**: by its name, or by `function::call` and
  `function::apply`, `onSomething.call(value)`, which give it the `this`
  they're given. Any other use as a value, a fn pointer, a `dyn Fn`, a
  generic `F`, `function::bind`, is a compile error, as JS might call it
  on what holds it where Rust gives none. A JS callback that reads its
  `this` is a `function::with_this` closure, the one way for that.
- **No closure that forwards to it is it, nor a forwarder its re-export**:
  `Box::new(|| on_something())` stays `() => onSomething()`, and `pub fn
  a() -> bool { inner::on_something() }` a function of its own (ADRs
  0605, 1064), which JS may call on what holds them.
- **Read only where its JS's `this` is what its JS caller gives**: the
  body of a function of the crate's that isn't generic. Anywhere else
  it's a compile error that says what to write:
  - in a closure, an arrow, whose `this` is the function's around it:
    read it there, `let this = js::this();`, or make the closure
    `function::with_this(..)`;
  - in a `function::with_this` closure, whose `this` is its first
    parameter: read that, one way for it;
  - in a method, whose `this` is its `self`, or its type of a static;
  - in a generic function, given dictionaries no JS caller gives;
  - outside a function, a thread-local's or a static's initializer.
- **A binding of its own, `#[link_name = "this"]` given nothing**, is
  refused as before: `js::this` is the one that reads the function's
  `this`, which the analysis checks where it's read.

## Alternatives

- **Read in a closure as the enclosing function's**, as JS's arrow sees
  it: a closure lowered to anything but an arrow, a `function` of
  preparation's, would read another, and a closure called after the
  function returned would still read it; `let this = js::this();` says
  which one plainly.
- **`function::apply` taking `Option<&Unknown>` for its `this`**: a
  change of the binding's signature, for each of its callers; `this.map(..
  as &dyn IntoUnknown)` is what a `with_this` closure writes today.
- **Allowing it as a value, each call through what holds it unbound**:
  `FnPointers` takes a crate's function for one that reads no `this`
  (ADR 0972, amended, rd579); a function JS is given would read the `this`
  JS gives, which a Rust caller of the same value doesn't. A `with_this`
  closure is the value that reads it.

## Consequences

- React DOM's root server entries print upstream's `.apply(this,
  arguments)`.
- ADR 1018's "a crate's or a library's free function never reads `this`"
  no longer holds: one that does is called through the namespace with
  none.

## What holds it

- `test/this.test.ts`:
  - run in JS: the entry's `apply(this, arguments)` called as a method,
    plainly, through `.apply` and `.call`, `null` too; Rust's calls, by
    name, through the namespace, through a forwarding closure and a
    forwarder, each `None`; `function::call`'s `this`;
  - each refusal, a binding of its own `this` too;
  - a library's function, which its consumer calls through its namespace
    with none, and whose use as a value is refused.
- Mutations `this-unread`, `this-in-closure-kept`, `this-in-with-this-kept`,
  `this-in-method-kept`, `this-in-generic-kept`, `this-reader-value-kept`,
  `this-reader-call-refused` (`analysis/arguments.rs`),
  `this-namespace-call-bound` (`mir/calls.rs`), `this-binding-any`
  (`binding_calls.rs`), `contract-this-unsaid` (`library.rs`), and
  `arguments-forwarded` (`passed_on.rs`), `forwarders-arguments-unasked`
  and `forwarders-library-arguments-unasked` (`pipeline/forwarders.rs`),
  which now hold both.
