# 1009. A constructor is held as a typed `js::Class`

Status: Accepted. The user's decision (2026-10-28, React port gap rd64:
option A, `Class<fn(A, B) -> T>`, and one way to hold a constructor).
Amends [0695](0695-new-of-a-function-value-is-jss-new.md), whose `js::new`
takes a typed class too;
[0696](0696-a-constructor-is-a-fn-item-where-its-written.md), whose `fn`
item is held as a `Class`, not as a value of any shape;
[0761](0761-a-rust-js-class-is-a-constructor-function-and-its-prototype.md),
whose `js::Class` takes its constructor's signature; and
[0972](0972-a-function-that-reads-its-this-is-with-thiss.md), whose
function is a constructor where `Class::of` holds it.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. The function,
its prototype and `new` are JS's own, upstream's; `Class::of` is the
function itself, and `new` of it makes what JS's `new` does: what the
function gives where that's an object, `this` otherwise. D, a compile
error, where it can't be a constructor: `Class::of` of a closure, an
arrow, which `new` throws a `TypeError` of.

## Context

React DOM's `SyntheticEvent.js` makes a constructor per interface:

```js
function createSyntheticEvent(Interface) {
  function SyntheticBaseEvent(reactName, reactEventType, targetInst, nativeEvent, nativeEventTarget) {
    this._reactName = reactName;
    ..
    for (const propName in Interface) { .. }
    ..
    return this;
  }
  assign(SyntheticBaseEvent.prototype, { preventDefault: function () { .. }, .., isPersistent: functionThatReturnsTrue });
  return SyntheticBaseEvent;
}
export const SyntheticEvent = createSyntheticEvent(EventInterface);
```

and each plugin constructs the one it holds, `new
SyntheticEventCtor(reactName, reactEventType, null, nativeEvent,
nativeEventTarget)`. rust-js had no form for it (gap rd64):
`#[rust_js::class]` is one constructor per struct, whose `fn new` can't
capture `Interface`, as no Rust item captures (E0434); a `fn` item is
the same; `function::with_this` gave the `function` and its `this`, but
as a `Box<dyn Fn>`, whose `.prototype` rustc won't let the program read
and return both, and `js::new` of it was a value of any shape, given any
arguments, making an `&Unknown`. Its `let`'s `#[rust_js::name]` was lost
where the function was moved into another `let`.

## Decision

**A constructor the program writes is held as a `js::Class`, typed by
its signature, `Class<fn(A, B) -> T>`:**

- **`Class::of(f)`** is the function itself, `link_name = "this"`, of a
  `fn` item, `Class::of(&fake)`, of one `function::with_this` makes,
  closed over what it captures, `Class::of(function::with_this(move
  |this, (a, b)| { .. this }))`, or of a `&dyn Fn` JS gives. Its type is
  its signature's (`js::Signature`): its parameters, and what `new` of it
  makes (`js::Made`), `T` where it gives `this`, a `&'static T`, as
  upstream's `return this;`; a value of any shape, `Unknown`, where it
  gives nothing or an `Option`, which `new` makes `this` of.
- **`js::new(class, (a, b))`** is `new class(a, b)`, given what the
  constructor takes, each argument checked by rustc as a call's is, and
  giving a `&'static T` (`js::Construct`). Of an untyped `Class`, a
  binding's global class or the crate's `js::class::<T>()`, and of a value
  of any shape, it's as before: rest arguments, a value of any shape.
  One `js::new`.
- **`Class::prototype()`** is `C.prototype`, an `&Unknown`, never
  `undefined` of a function the program writes: `Object.assign(
  SyntheticBaseEvent.prototype, { .. })`, where `js::get` of it gave an
  `Option` to unwrap.
- **`Class` defaults to `Class<AnyConstructor>`**, today's untyped one,
  so `js::class::<T>()` and a held binding class, `&'static Class`, are as
  they were, and constructed as they are, `js::new(class, ())`.
- **Its declaration is a constructor's type**, `new (value: string) =>
  BaseSyntheticEvent`, which TypeScript's `new` checks as rustc does; the
  model's function type takes `construct` (`@rust-js/typescript`).
- **A `let`'s `#[rust_js::name]` names the variable it's moved into**
  too: `let made = synthetic_event;` of one named `SyntheticEvent` is
  that one variable, by the name the program gave (`Locals::given_name`, asked
  by tidy's `aliases`), as a function declared of it is, `function
  SyntheticBaseEvent(..)`.

**One way.** A constructor held as a value of any shape is refused,
naming the form (`classes::reject_constructors_as_values`): a function
constructed, `js::new(js::unknown(&fake), ())`, or whose prototype is
read, `js::get(js::unknown(&fake), "prototype")`, of `Class::of(&f)` and
`.prototype()`; a `Class` constructed as any value, `js::new(
js::unknown(class), ())`, of `js::new(class, ..)`; and `Class::of` of a
closure, an arrow. A function as a value of any shape stays
`js::unknown(&f)` for what any value does, its properties set, compared.

So React's port writes:

```rust
fn create_synthetic_event(
    #[cfg_attr(rust_js, rust_js::name = "Interface")] interface: &'static EventInterfaceType,
) -> &'static SyntheticEventClass {
    #[cfg_attr(rust_js, rust_js::name = "SyntheticBaseEvent")]
    let synthetic_base_event = Class::of(function::with_this(
        move |this, (react_name, react_event_type, target_inst, native_event, native_event_target): (..)| {
            // Safety: `new` gives its constructor the event it makes.
            let this: &'static BaseSyntheticEvent = unsafe { js::cast(this) };
            ..
            this
        },
    ));
    assign(synthetic_base_event.prototype(), (&SyntheticEventPrototype { .. },));
    synthetic_base_event
}
```

## Alternatives

- **A `#[rust_js::class]` whose constructor is a closure the factory
  makes** (option B): a typed struct literal and `&self` methods, but a
  class of no `new` whose module-level impl is printed inside the
  factory, once per constructor; upstream's `assign(X.prototype, {..})`
  would be `X.prototype.m = function` per method; and a struct literal
  sets every field first, so the fields upstream sets after its
  interface loop would be `Option`s or JS's property order would change.
- **`Class<(A, B), T>`**: two parameters where a constructor's type is
  one, read as `Box<dyn Fn(A, B) -> T>` is.
- **A `fn` item and `.bind(null, Interface)`**: one prototype for every
  event, a results change.
- **A new `function::constructor`**: a third way to write a constructor.
- **Typing `T` by an annotation alone**: `Class::of` would claim what any
  function makes, a cast with no `unsafe`; `T` is what the function gives.

## Consequences

- ADR 0696's `js::new(js::unknown(&fake), ())` and `js::get(js::unknown(
  &fake), "prototype")` are `js::new(Class::of(&fake), ())` and
  `Class::of(&fake).prototype()`; ADR 0911's `js::new(js::unknown(*class),
  ())` is `js::new(*class, ())`. Migrated: the guide, test/js-object's,
  test/webapi-bindings', and test/ports/component-stack-frame, React's
  ReactComponentStackFrame. react-in-rust's own uses migrate with it.
- A function `Class::of` holds is still a function, `js::unknown(&f)` of
  it any value; `Reflect.construct(f, ..)` takes it as it is.
- `rust_js::name` on a `let` is kept where tidy merges the `let` with
  another; one MIR merges two bindings of keeps the first its debug info
  lists, as before.

- **Making a constructor runs none of it**: `function::with_this(closure)`
  makes a function of its closure, which runs only where it's called, a
  call of a `dyn Fn` (`cell_borrows::binding_runs_nothing`). So React's
  `export const SyntheticEvent = createSyntheticEvent(EventInterface);`, a
  `pub` thread-local made as its module loads (ADR 1006), whose
  constructor calls an interface's fn pointer, is made there, not refused
  as one whose initializer may run what rust-js can't follow.
- `js::new` is generic, so a cast given it names its type,
  `js::new(unsafe { js::cast::<&Unknown>(ctor) }, ..)`, as React's
  ReactFiberBeginWork's must: rustc can't infer which constructor a bare
  `js::cast` is.

## What holds it

- `test/js-constructors.test.ts`: createSyntheticEvent as the port writes
  it, its JS upstream's, run in Node (two interfaces' constructors and
  prototypes apart, an event's own keys in upstream's order, its
  prototype's methods reading `this`, `instanceof`); `js::new` of a held
  class from Rust, an alias's given name kept; rustc's errors of wrong
  arguments; the declaration's `new (..) => T`; and each refusal.
- Mutations `constructor-as-unknown-constructed`,
  `constructor-as-unknown-prototype`, `class-as-unknown-constructed`,
  `class-of-a-closure` (`classes.ts`), `alias-of-a-given-name-renamed`
  (`mir/tidy/bindings.ts`), `given-let-names-unsaid` (`mir/locals.ts`),
  `typed-class-declared-a-function` (`declarations.ts`),
  `with-this-runs-its-closure` (`analysis/cell_borrows.ts`).
