# 0678. A parameter's leading `_` isn't in its JS name

Status: Accepted. Amends [0573](0573-every-rust-name-is-camel-case-in-js.md),
whose names kept a leading `_`: a parameter's is Rust's mark of one
unused, which its JS name and its declaration's are without. Amended:
a closure's parameter meets the bindings of the bodies it's in and its
own, not a sibling closure's. Amended: a parameter the body reads keeps
its `_`.

Case ([0262](0262-when-rust-and-js-disagree.md)): A. A parameter's name
changes nothing a program does, and where it would, another binding's
of the same name, it stays as Rust writes it.

## Context

React's `describeClientReference(ref)` (shared/ReactSerializationErrors.js)
doesn't read its parameter. Its port writes `fn
describe_client_reference(_ref: &Unknown)`, as rustc's `unused_variables`
asks, and rust-js wrote `function describeClientReference(_ref)`: the `_`
is Rust's, for its lint, and no JS reader's. The user decided
(2026-10-23): a parameter's leading `_` doesn't reach JS.

## Decision

**A parameter's JS name is its camelCase name without a leading `_`**,
the one place it's made, `lower::parameter_name`, which a body's
parameter and its function's declaration both ask:

```rust
fn describe_client_reference(_ref: &Unknown) -> String {
    "client".to_string()
}
```
```js
function describeClientReference(ref) {
  return "client";
}
```

- **Where what's left is a word JS keeps, it stays**: `_new`, not
  `new$`, which reads as no name a person would write.
- **Where another binding of the function has that name, it stays**:
  `fn beside(_x: u32, x: u32)` is `beside(_x, x)`, not `x` and `x$1`; a
  closure's binding, inside the function or around a closure's
  parameter, too: `move |base| base + _base` keeps `_base`, which the
  closure's `base` would shadow.
- **A closure's parameter is one too**: `|_event, count| count + 1` is
  `(event, count) => (count + 1) >>> 0`.
- **Only a parameter**, a function's or a closure's, as the user
  decided; a `let _guard` keeps its name.

## Why

- **The `_` is Rust's lint's**: `unused_variables` asks for it, and JS
  has no such lint to answer; upstream writes `ref`.
- **One place**: the declaration names a parameter as its JS does, so a
  TypeScript caller sees `ref` in both.

## Alternatives

- **Every binding's `_`**, a `let`'s too: the decision was of
  parameters; a `let _guard = lock()` is kept for what it drops.
- **`ref` beside `x` renamed, `x$1`**: a rename where Rust's name reads
  as well.

## Consequences

- Seven corpus cases and a snapshot drop a parameter's `_`:
  `keep(f, dropF)`, `oddPartialEq_eq(odd, other)`, `pass(t)`.

## What holds it

- `test/lowering-names.test.ts`, "a parameter's leading underscore isn't
  in its JS name", its JS and its declarations, run.
- Mutations in `scripts/mutations/lower.ts`,
  `scripts/mutations/lower/mir/locals.ts` and
  `scripts/mutations/lower/declarations.ts`.

## Amendment: a closure beside it isn't its function

React's `ReactNoopUpdateQueue` is a static of closures, `isMounted:
|_public_instance| false` beside `enqueueForceUpdate: |public_instance,
_callback, _caller_name| ..`. The static's value is one body, and each
binding of it, a sibling closure's too, counted as another of the
parameter's function: `_publicInstance` stayed, where upstream writes
`isMounted: function (publicInstance) { return false; }`.

**A parameter's name meets the bindings it can**: its function's own,
of the closures in it too, and those of each body around it outside
another closure; a closure beside it, in the same function or static,
is a scope of its own, whose bindings it never sees nor they its
parameters. `|_instance| false` beside `|instance, _callback| instance`
is `(instance) => false`. Around it still counts: `move |_total| total`
keeps `_total`, and a closure in it, `|_v| { let g = |v| v + 1; .. }`,
keeps `_v`.

- Case ([0262](0262-when-rust-and-js-disagree.md)): A, as before: no
  binding beside the closure is in its scope, so no name it reads
  changes.
- Held by `test/lowering-names.test.ts`'s "a parameter's leading
  underscore isn't in its JS name" (a static's sibling closures, a
  closure in a closure) and the mutations
  `parameter-closure-bindings-unseen`,
  `parameter-sibling-closure-bindings-seen` and
  `parameter-inner-closure-bindings-unseen`.

## Amendment: a private function's last `_` parameters are left out

The user's decision (2026-10-27). React's findNextSibling(child) is a
visitor its traversal calls with three more arguments it doesn't take;
its port, `fn find_next_sibling(child: u32, _: &(), _: &(), _: &())`,
whose type the visitor's `dyn Fn` asks, printed `function
findNextSibling(child, _, _$1, _$2)`.

**A function the crate doesn't export, a `fn` that isn't `pub`, leaves
out its last parameters that are `_`**, named nothing and read nowhere
(`last_params`): JS ignores what a call gives past a function's own
parameters, and the crate's calls and its function values give them as
before. Not of one given anything after them, a generic function's
dictionaries, `fn id<T: Any>(_: &T)`'s `id(_, TAny)`, or a formatter's
`options`, whose places they hold; not of a method or a closure, whose
arity JS may read; and not of an exported function, whose `.d.ts` and
`length` are its API. A parameter the program named, `_a`, stays, `a`.

Case: N ([0262](0262-when-rust-and-js-disagree.md)) to the program,
which never reads a function's `length`; JS that reads a private
function's `length` can't reach it.

Held by `test/lowering-parts.test.ts`'s "a private function's last
unnamed parameters are left out", run in JS; the corpus's `dyn_any.rs`,
`type_facts.rs` and `closure_type_ids.rs` for the dictionaries after
them; and the mutations `unnamed-last-params-kept`,
`unnamed-last-params-of-generics-too` and
`unnamed-last-params-of-exported-too` (`lower/mir.ts`).


## Amendment: a parameter the body reads keeps its `_` (2026-10-28)

The user's decision. React's `mountLazyComponent(_current, workInProgress,
elementType, renderLanes)` reads its `_current`, and the port's `fn
mount_lazy_component(_current: Option<&Fiber>, ..)`, which reads it too,
printed `function mountLazyComponent(current, ..)`. A `_` the body reads
past isn't Rust's mark of one unused, which `unused_variables` asks for:
it's the program's name.

**A parameter's `_` goes only where the body doesn't read it**
(`reads_parameter`): a path to its binding in the function's body, a
closure's in it too, `|| _seen`; an `async fn`'s move of it into its
future, `let _arg = _arg;`, which no program writes, is no read, and the
binding it makes is the parameter; of a library's function its consumer
compiles from MIR alone (ADR 0914), a use of its local, so both crates'
copies name it alike; not its drop, which MIR writes where the source
writes none (`pick(a, b, dropB)`, of `fn pick<T, U>(a: T, _b: U) -> T`).

```rust
pub fn current_or(_current: Option<u32>, fallback: u32) -> u32 {
    _current.unwrap_or(fallback)
}
```
```js
export function currentOr(_current, fallback) {
  return _current ?? fallback;
}
```

- Case ([0262](0262-when-rust-and-js-disagree.md)): None, as before: a
  name changes nothing a program does.
- Held by `test/lowering-names.test.ts`'s "a parameter's leading
  underscore isn't in its JS name" (`read(_current, _seen)`, a closure's
  read), `test/crates.test.ts`'s "a library's Copy bound takes a copy
  only where one is given" (`currentOr(_current, fallback, TCopy)` of the
  consumer's MIR), the corpus's `future_sizes.rs` (`async function
  test(arg)`), and the mutations `parameter-read-unmarked`,
  `parameter-read-in-closure-unseen`, `parameter-async-move-read` and
  `parameter-read-of-mir-unseen`.
