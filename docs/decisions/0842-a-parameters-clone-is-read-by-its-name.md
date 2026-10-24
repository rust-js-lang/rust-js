# 0842. A parameter's clone that is the same JS value is read by the parameter's name

Status: Accepted. Extends tidy's `aliases`, of
[0320](0320-counted-rcs.md)'s uncounted `Rc`, which is the value itself.

Case ([0262](0262-when-rust-and-js-disagree.md)): A. A `clone()` whose JS
is the value itself, `const resolved = payload`, gives the variable the
same value the parameter holds; where nothing gives either name another
value, reading one or the other reads the same thing.

## Context

React's `lazy` shares its payload with the two callbacks its thenable
settles. Rust moves an `Rc`'s clone into each `move` closure:

```rust
fn lazy_initializer(payload: &Rc<Payload>) -> Option<&'static Unknown> {
    let resolved = Rc::clone(payload);
    let rejected = Rc::clone(payload);
    thenable.then(
        move |module_object| resolved.status.set(Status::Resolved),
        move |error| rejected.status.set(Status::Rejected),
    );
```

An uncounted `Rc` is the object itself, so its clone was a variable of its
own, `const resolved = payload;`, and each callback read the alias.
Upstream's JS, which needs no clone to share an object, reads `payload`.
The playground wrote the same of a `String`'s clone given to a handler,
`const opened = path; .. onClick={() => onOpen(opened)}`.

## Decision

**`const b = a;` that a `clone()` made, of a parameter `a` nothing assigns
or declares again, is `a`, read wherever `b` was, where nothing declares
or assigns `b` again after.**

| Rust | JS |
|---|---|
| `let resolved = Rc::clone(payload); move \|v\| resolved.result.set(v)` | `(v) => { payload.result = v; }` |
| `let opened = path.clone(); move \|\| on_open(opened)` | `() => onOpen(path)` |
| `let first = payload.clone(); payload = other;` | `const first = payload; payload = other;` |
| `let gas = n;`, a move, not a clone | `const gas = n;` |

- **MIR says which are clones** (`hints.rs::clone_names`): the locals a
  call of `Clone::clone` writes, which tidy is told (`Hints::clones`). A
  clone whose JS copies, `[...items]` or `$rcClone(rc)`, isn't a
  variable's `const b = a` and stays.
- **Only a parameter's**: a parameter holds one value for its function
  where nothing assigns it, so `b`, given it, is it throughout.
- **A name the program gave a move stays**: `let gas = n;` is the
  program's choice of a second name. A clone's is the name Rust needs to
  give a `move` closure its own handle, which JS's closure doesn't.

## Why

- **The same value, read by its name**: the alias and the parameter hold
  one object, a string or a number JS never changes in place; neither is
  given another, so every read of one is a read of the other.
- **It's upstream's JS**, React's `payload` read in its callbacks.

## Alternatives

- **Every `const b = a;` of a parameter is `a`**: loses the names the
  program gave its moves (`clone-alias-of-any-const`). Rejected.
- **The port shares `&'static Payload`, `Box::leak`ed**: a leak the port
  rules forbid, and not the Rust a person writes. Rejected.

## Consequences

- The playground's `FileItem` and `Toolbar` read `path` and `onCompile`
  in their handlers, where they read `opened` and `onTest`.

## What holds it

- `test/corpus/rc_shared_by_closures.rs`: two closures each given a
  clone of a parameter's `Rc`, and a parameter given another value after
  its clone is made, whose clone keeps its name, with native Rust.
- Mutations in `scripts/mutations/lower/mir/hints.ts`
  (`clone-names-unfound`) and `scripts/mutations/lower/mir/tidy/bindings.ts`
  (`clone-alias-kept`, `clone-alias-of-a-reassigned-parameter`,
  `clone-alias-of-any-const`).
