# 1007. A library's own fn pointers are Rust functions to its consumers

Status: Accepted. Amends [0907](0907-a-fn-pointer-runs-what-the-crate-made-it-of.md)
(a fn pointer type whose signature holds a `dyn` is keyed too) and
[0330](0330-undefined-given-to-js.md) (a call through a library's own fn
pointer leaves a trailing `undefined` out). React port gap rd56.

Case ([0262](0262-when-rust-and-js-disagree.md)): A. A Rust function never
counts its arguments, so `f(x)` and `f(x, undefined)` are the same call
of one; only a JS function can tell them apart, and none is called so.

## Context

React's `Children` is a struct of its own functions, read by React DOM's
ReactDOMOption through the `react` crate's thread-local, as upstream's
`Children.forEach(props.children, function (child) { .. })`:

```rust
type ForEachChildren = fn(Option<&'static Unknown>, &dyn Fn(Option<&'static Unknown>), Option<&'static Unknown>);

CHILDREN.with(|children| (children.for_each)(js::get(props, "children"), &|child| { .. }, None));
```

It printed `Children.forEach(props.children, (child) => { .. }, undefined)`:
a call through a fn pointer keeps its trailing `undefined` unless the
pointer may only be the crate's own functions (ADR 0330, amended). Two
things made it "anything":

1. **A signature holding a `dyn` had no key** (`fn_pointers::key`), so
   nothing was known of its values, in one crate too.
2. **A library's static is a value from outside**, whose fn pointers may
   be any function, to a consumer that can't see what the library made
   them of.

## Decision

- **Every fn pointer type is keyed, its lifetimes erased, those its
  signature binds too** (`fn_pointers::key`'s `Lifetimeless` folder):
  `fn(&dyn for<'a> Fn(&'a u32))` and `fn(&dyn Fn(&'static u32))`, which
  subtyping may make one of the other, are one key, so what's made of one
  is known of the other.
- **A library's contract says which of its structs' fields hold only Rust
  functions** (`FieldFact::RustFunctions`): each fn pointer the field may
  hold is a type no value of which comes from outside it, made only of its
  own functions and closures, none a binding's (`FnPointers::rust_fields`,
  of `rust_only`).
- **A consumer reading such a field's value, a library static's say,
  takes its fn pointers as Rust functions** (`FnPointers::library_rust`),
  where it took them as anything: `rust_only` holds of them, so a call
  through one is rust-js's own, and its trailing `undefined` after another
  argument is left out (`mir/dispatch.rs`), `Children.forEach(children,
  fn)`. What they run is still the library's, which the consumer can't
  see: `may_be` says anything of them, so its other analyses (the cells a
  call may borrow, a thread-local made where it's first used) are as
  before.
- **Whether a call is rust-js's own is `FnPointers::rust_only`**, the
  owner's answer, which `mir/dispatch.rs` asks; it no longer reads
  `may_be`'s list itself.

## Why

- It's upstream's call, and the same program: React's `forEach` is a Rust
  function, which never asks how many arguments it was given.
- A field's fact, not a type's: the contract names fields by their keys
  already (ADR 0906), while a type of the library's has no name a
  consumer shares, and the struct whose value is read is what says where
  the pointers come from.

## Alternatives

- **A fact of each fn pointer type**: one a consumer's types would have to
  be named in, printed paths that differ between the crates.
- **A fact of the static**: no help where the struct reaches the consumer
  otherwise, and the field is what's read.

## Consequences

- A field of a library's struct that a pub function takes or gives a fn
  pointer of its type to, `pub fn hooks(on_each: fn(..)) -> Hooks`, is no
  such field: a consumer's call through it keeps its `undefined`.
- `tooling/manifest.js` accepts the fact, as it does each field fact.

### What holds it

- `test/runtime-package.test.ts`, "an undefined given to JS is an
  argument": a static's field of a `fn(f64, &dyn Fn(f64) -> f64,
  Option<f64>)`, `CHILDREN.forEach(1, (x) => x * 2)`, and one given by a JS
  caller, `f(1, (x) => x, undefined)`, run in node.
- `test/crates.test.ts`, "a library's field of its own functions is called
  as the crate's own", and `two crates: fn_pointer_of_a_library` against
  native Rust: `children.forEach("ab", (i) => ..)`, a field given from
  outside keeping `undefined`, and a consumer's thread-local whose
  initializer calls one, made where it's first used.
- Mutations `pointer-of-a-dyn-signature-unkeyed`,
  `pointer-given-from-outside-left-out`, `binding-pointer-undefined-left-out`,
  `library-rust-fields-unwritten`, `library-rust-field-read-as-open`,
  `library-rust-pointer-runs-nothing` (`fn_pointers.rs`) and
  `own-pointer-given-undefined` (`mir/dispatch.rs`).
