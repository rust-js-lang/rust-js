# 0775. A `()` given to JS is `undefined`; a list of none is `[]`

Status: Accepted. Amends [0020](0020-structs-and-tuples.md), whose `()`
given to JS was its tuple's array, `[]`; [0443](0443-a-value-a-caller-may-leave-out-is-an-option.md)'s
point 6, whose `()` as a `dyn` JS is given the value of was `[]`; and
[0225](0225-unknown-values.md)'s amendment, whose `undefined` was of a
structured clone's `()` alone. Fixes a wrong answer of
[0721](0721-strict-equals-is-jss-triple-equals.md)'s `js::strict_equals`.

Case: C ([0262](0262-when-rust-and-js-disagree.md)): results changed.
`x === []` is never true, so `js::strict_equals(x, ())` answered `false`
of every `x`, where Rust's meaning, and JS's `undefined`, has it `true`
of `undefined`.

## Context

**A wrong answer.** React's port compares a value with nothing,
`js::strict_equals(x, ())`, for upstream's `x === undefined`. rust-js
wrote `x === []`: a new array, never `===` anything, so the test was
always `false`. `()` is `undefined` everywhere else rust-js writes it: a
function that returns nothing returns it, `vec![(), ()]` is
`[undefined, undefined]`, `Some(())` `$some(undefined)`, and the guide's
table of types says so. Only where a `()` crossed into a binding was it
`[]`:

| Where | Was | What JS got |
|---|---|---|
| a binding's parameter of any value, `impl IntoUnknown`, a `T` | `[]` | `x === []`, `o.k = []`, `typeof []`, `"object"` |
| a `()` as a `dyn` of a binding crate's or a marker's trait, `&dyn IntoUnknown`, `&dyn Any` | `[]` | `f.call([])` |
| a parameter whose type is `()` itself, and a binding taken as a value | `[]` | `(a, b) => kinds([], b)`, its own argument dropped |
| an `init(arg)` given `()`, `use_reducer_with(reduce, (), init)` | `[]` | the closure's `()` a `[]` |
| a structured clone's (ADR 0225 amended) | `undefined` | |
| a `()` given by reference, `&()`, to `impl IntoUnknown` | `undefined` | |

**How it slipped through.** ADR 0020 made a tuple an array and gave a
`()` to JS as the tuple of none, `[]`, so that `use_effect(f, Some(&()))`
is `useEffect(f, [])`. ADR 0225's amendment met the same `[]` in
`postMessage([])` and fixed it for a structured clone's parameter alone,
leaving `[]` the default: two rules, in three places (`mir/calls.rs`,
`traits/dyns.rs`, `binding_calls.rs`), each right of the cases it was
written for. No test gave a `()` to a parameter of any value and ran it:
ADR 0721's tests of `strict_equals` compared `Option`s, text and numbers,
and the corpus has no binding of any value, so `x === []` was never run.

## Decision

**A `()` given to JS is `undefined`, what `()` is everywhere, given as it
is: by value, by reference, as a `dyn`, held in a variable, or given a
binding taken as a value. Where it's a list of items, whose tuple is its
array, it's the list of none, `[]`**: rest arguments, a variadic
binding's last parameter or a `&dyn js::Rest<T>`, spread, so none is
given; and a hook's dependencies, `react::DependencyList`, by value or as
a `dyn`, `useMemo(f, [])`, which runs once where `undefined` would run it
after every render.

`representation::unit_given` is the one answer, of a binding's parameter or of
a `dyn`'s trait, which `mir/calls.rs::binding_arg` and
`traits/dyns.rs::unsize_trait` ask; `binding_calls.rs`'s structured clone
rule is gone, as the default is its answer.

```rust
pub fn is_unit(x: Option<&Unknown>) -> bool {
    js::strict_equals(x, ())
}

pub fn set_unit(o: &Unknown) {
    js::set(o, "k", ());
}

pub fn Once() -> JSX::Element {
    let doubled = use_memo(|| 2, ());
    ..
}
```

```js
export function isUnit(x) {
  return x === undefined;
}

export function setUnit(o) {
  o.k = undefined;
}

export function Once() {
  const doubled = useMemo(() => 2, []);
  ..
}
```

## Why

- **It's right**: `()` is `undefined` in JS, so `strict_equals(x, ())`
  is `x === undefined`, as the native corpus case's `===` answers.
- **One way**: one representation of `()`, the one the guide states, and
  one owner of the one place it's otherwise, a list's.
- **A list is what's a list**: rest arguments and dependencies are a
  tuple of items, `(a, b)`'s `[a, b]`, and their `()` is the empty one.

## Rejected

- **`[]` everywhere a `()` crosses, as before**: wrong of every value
  of any shape, and unlike `()` anywhere else.
- **Refusing `()` as a value of any shape**: `undefined` is JS's value of
  it, and a port writes it; a refusal would ask for `None::<&Unknown>`,
  a type of nothing.
- **A `rust_js` attribute to mark a list's trait**: one trait,
  `DependencyList`, needs it; `js::Rest` is already marked
  `#[rust_js::rest]`. Recognition names `DependencyList` as it names
  the js crate's traits.

## Consequences

- `use_reducer_with(reduce, (), init)` is `useReducer(reduce, undefined,
  init)` (todomvc's snapshot), where it was `[]`: `init` is given
  `undefined`, what its `()` is.
- A `()` as a `dyn Any` or a marker's `dyn` holds `undefined`, its
  `value` left out of its pair.
- Held: `test/corpus/unit_given_to_js.rs`, run against native Rust;
  `test/unit-given.test.ts`, which runs each shape in Node, rest
  arguments and dependencies too; mutations `unit-given-binding-array`,
  `unit-dyn-given-array`, `unit-given-rest-not-a-list`,
  `unit-given-dependencies-not-a-list`, `unit-given-bound-not-a-list` and
  `unit-given-dyn-not-a-list`.
