# 0846. A parameter's clone that is the same JS value is read by the parameter's name

Status: Accepted. Extends tidy's `aliases`, of
[0320](0320-counted-rcs.md)'s uncounted `Rc`, which is the value itself. Extended by [0879](0879-an-if-and-a-closures-captures-read-as-written.md): a clone of what a closure captured by value, which its maker never assigns again, is read by its name too.

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

## Amendment: a clone out of a cell's borrow, read once next (rd189; superseded below)

React's FragmentInstance clones its fiber out of its cell before another
crate's call, as idiomatic Rust does, so no borrow is held across it:
`let fragment_fiber = Rc::clone(&self.fragment_fiber.borrow());
traverse_fragment_instance(&fragment_fiber, ..)`, which printed `const
fragmentFiber = this.fragmentFiber; traverseFragmentInstance(fragmentFiber,
..)`, where upstream reads `this._fragmentFiber` there. **A `const` a
`clone()` made of a field read, of `this` or of a parameter nothing
assigns, read once, as an argument of the next statement's call before
anything there that may do something, is that read in its place** (tidy's
`aliases`, `field_of`, `first_read_as_argument`): the field is read where
it was, before the call. One the cell's written before, a statement
between, or read after an argument that does something, is held as
before. A `js::cast` struct's field (rd204) is read once either way. Case
A: the same read, at the same time. Held by
`test/corpus/cell_clone_read_in_place.rs` against native Rust and the
mutations `cell-clone-held`, `cell-clone-after-an-effect` and
`cell-clone-not-next` (`mir/tidy/bindings.ts`).

## Amendment: the program's `let` of such a clone stays (rd213)

The rd189 amendment read a clone-made `const` of a field in its place,
`traverseFragmentInstance(this.fragmentFiber, ..)`, as React's
FragmentInstance upstream does; but React names the same shape elsewhere,
SuspenseContext's `const handlerOnStack = suspenseHandlerStackCursor.current;
push(..)` and updateContainerSync's `const current = container.current;`,
each a `let` the Rust writes too, `let handler_on_stack =
cursor.current.borrow().clone();`, so the rule removed a name the program
gave, and, inside a thread-local's `with`, the comments before it. Nothing
in the Rust tells the two apart: each clones out of a borrow so as not to
hold it across a call. **So a `let` the program wrote of a field's clone
stays, named as written; the rd189 rule is gone.** FragmentInstance's
`const fragmentFiber = this.fragmentFiber;` is a port difference, the
user's to accept. Held by `test/corpus/cell_clone_read_in_place.rs`, its
`const fiber = holder.fiber;`.
