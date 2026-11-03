# 1048. A function that reads `js::arguments()` is given each argument

Status: Accepted. The user's decision (2026-11-01, rd332, option A). Amended (rd509): a `function::with_this` function's own. Amended (#553): a library's, called by its consumer. Extended by [1081](1081-a-function-reads-its-own-this-by-js-this.md): `js::this()`, a function's own `this`, by the same rules, which its owner, `analysis/arguments.rs`, finds too.
Extends [0313](0313-console-forms.md)'s and
[0443](0443-a-value-a-caller-may-leave-out-is-an-option.md)'s trailing
`undefined`s, and [0605](0605-an-arrow-that-passes-on-has-one-owner.md)'s
forwarding closures, each of which a function that reads its arguments
now stops.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change,
preserve Rust's. A function that reads `arguments` counts what its caller
gives it, so each rule that leaves an `undefined` out, or makes a closure
the function it calls, would change what it reads. For these functions
neither applies.

## Context

react-art's fill classes keep every argument they're given and pass them
on:

```js
const slice = Array.prototype.slice;

class LinearGradient {
  constructor(stops, x1, y1, x2, y2) {
    this._args = slice.call(arguments);
  }
  applyFill(node) {
    node.fillLinear.apply(node, this._args);
  }
}
```

The port wrote `#[rust_js::rest] args: &[Option<&Unknown>]`,
`constructor(...args)`: `length` 0 and no names, which the user accepted
for a while as an ADR 0262 difference. Five `Option<&Unknown>`
parameters drop a sixth argument and give five where `arguments` may
hold two. ReactFlightReplyClient and the reconciler's `Dispatch`
callback have the same shape.

## Decision

**`js::arguments()` is JS's `arguments`, TypeScript's `IArguments`
(`js::Arguments`): each argument the function it's read in was called
with.** With `js::array::SLICE`, `Array.prototype.slice` as a value, and
a binding's method read as a value by a getter, the port writes:

```rust
use js::array::SLICE as slice;

#[cfg_attr(rust_js, rust_js::class)]
impl LinearGradient {
    pub fn new(_stops: Option<&Unknown>, _x1: Option<&Unknown>, ..) -> Self {
        LinearGradient {
            args: function::call(slice, Some(js::arguments()), ()),
        }
    }

    pub fn apply_fill(&self, node: &Node) {
        function::apply(node.fill_linear(), Some(node), self.args.map(|a| a as &dyn js::IntoUnknown));
    }
}
```

```js
export function LinearGradient(stops, x1, y1, x2, y2) {
  this.args = Array.prototype.slice.call(arguments);
}

LinearGradient.prototype.applyFill = function (node) {
  node.fillLinear.apply(node, this.args);
};
```

- **Each call rust-js writes of it gives every argument the Rust does**,
  an `undefined` too, `new LinearGradient(undefined, undefined, ..)`: a
  direct call, one through a `fn` pointer that may be it, and a library's
  consumer's, by its contract's `arguments` fact (`ItemFact::Arguments`).
  One owner, `analysis/arguments.rs`, whose set `FnCx::reads_arguments`
  answers, and the calls ask it.
- **No closure that forwards to it is it**: `|a, b| given(a, b)`, a `fn`
  that gives it less, `|first, _| one(first)`, and a bound function stay
  arrows (`passed_on`), since the function would count what its JS caller
  gives it.
- **Read only where its JS's `arguments` is the Rust's**: a function of
  the crate's that isn't generic, a class's constructor, method or
  static, or a namespace's function. Anywhere else it's a compile error
  that says what to write instead:
  - in a closure, whose arrow's `arguments` is the enclosing function's;
  - in a generic function, which is given its type parameters'
    dictionaries too;
  - in a trait's method, which is called through its dictionary;
  - in a method of a type that isn't a class, which is given its value
    first;
  - in an object's method.
- **A binding's method as a value** is its getter, `#[link_name = "get
  fillLinear"] fn fill_linear(&self) -> &'static dyn Fn(..)`, as typed as
  the method. The user chose it over a `js::method(&node, "fillLinear")`
  of a string.

## Consequences

- `Array.prototype.slice.call(arguments)`: a private `use js::array::SLICE
  as slice;` is read in place, not upstream's module `const slice =
  Array.prototype.slice;`. ADR 0622 makes a global binding the module's
  `const` only where a `pub use` or its default names it. Whether a
  private `use .. as` does too is a question for the user: it would
  change other ports' output, `use js::array::is_array as is_array_impl;`
  too.
- react-art's fill classes are still constructor functions, ADR 0761's
  form for a class that extends nothing, as the user decided.

## What holds it

- `test/arguments.test.ts`:
  - the fill class, run in Node with more and fewer arguments, its
    `length` 5, a Rust caller's five `undefined`s, a pointer's call, a
    forwarding closure, one that captures, a `fn` that gives less, and a
    bound function;
  - each refusal;
  - a library's function, which its consumer calls with every argument.
- Mutations `arguments-unread`, `arguments-in-closure-kept`,
  `arguments-in-generic-kept`, `arguments-in-trait-method-kept`,
  `arguments-in-plain-method-kept` (`analysis/arguments.rs`),
  `arguments-own-call-left-out` (`mir/calls.rs`),
  `arguments-pointer-left-out` (`mir/dispatch.rs`), `arguments-forwarded`
  (`passed_on.rs`) and `contract-arguments-unsaid` (`library.rs`).

## Amendment: a `function::with_this` function's own (rd509)

ReactFlightReplyClient's server reference `bind` and both `action`s read
`arguments`, and they close over `id`, `callServer` and `bound`, so the
refusal's "read it in the function around" can't apply. A closure
`function::with_this` makes is a JS `function`, not an arrow, so its
`arguments` is its own. **`js::arguments()` is read directly in its body**,
and each call of a `dyn Fn`, a generic one or a pointer, of the same
parameters' tuple gives every argument (`ReadsArguments::may_call`, in
`mir/calls.rs`). A forwarding closure to one of those isn't it either
(`passed_on`; an array method's generic callback, given the index and the
array too, is the case that matters). Case: as above, preserve Rust's.

Held by `test/arguments.test.ts`' `counter`, called by JS with two
arguments and by Rust with one `None`, and forwarded to, and the mutations
`with-this-arguments-refused` (`analysis/arguments.rs`) and
`with-this-call-left-out` (`mir/calls.rs`).

## Amendment: a library's `with_this` function, called by its consumer (#553)

The amendment above left a library's `with_this` function that reads
`arguments` to "not yet", and nothing refused it: a consumer's call of
one through its `dyn Fn` left its trailing `None`s out, the wrong answer
of #553. `dep::counter()(None, None)` gave 2 in the library,
`counter()(undefined, undefined)`, and 0 in its consumer,
`Dep.counter()()`; `f(Some(x), None)` gave 1, and a field's `(h.f)(None,
None)` 0. Case: results change; preserve Rust's.

**A library's contract names the parameters' tuple of each `with_this`
function it makes that reads `arguments`** (`with_this_arguments`, a
shape each, as `counted_rcs` names a type, ADR 0852), and a consumer's
call of a `dyn Fn`, a generic `Fn` or a pointer that may be of one of
them, `may_be_shape`, gives every argument, as the library's own does
(`ReadsArguments::may_call`, asking `Foreign::may_call_reading_arguments`).
A consumer is given every library's contract, so a library that hands on
another's function says only its own. A call of other parameters is as
before, `f()` of a `dyn Fn(Option<&Unknown>)`: no such function is of its
type.

`js::arguments()` has no native Rust behavior to compare with, so
`test/arguments.test.ts`' "a library's function::with_this function that
reads arguments is given each by its consumer" runs the four shapes in
Node, beside the library's own call: through the function's value, a
`let`, a struct's field and a generic `Fn`, each 2. Mutations
`contract-with-this-arguments-unsaid`,
`with-this-arguments-unread-by-consumer` and
`with-this-arguments-any-shape` (`library.rs`).

