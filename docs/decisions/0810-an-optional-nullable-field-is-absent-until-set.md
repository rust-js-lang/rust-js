# 0810. An optional nullable field is absent until it's set

Status: Accepted. Extends [0275](0275-nullable-writes-null.md) and
[0772](0772-a-nullable-cell-field-writes-null.md), whose nullable field's
`None` was `null` where its struct is made too. Amended: given another
optional field's value, it's that value as it is, below.

Case ([0262](0262-when-rust-and-js-disagree.md)): A: an absent key,
`undefined` and `null` are each `None` to the program, which reads any of
them as one (ADRs 0030, 0280); only what JS sees of the object changes.

## Context

React's `createContext` makes its object without its DEV-only fields and
adds them in DEV:

```js
const context: ReactContext<T> = {
  $$typeof: REACT_CONTEXT_TYPE,
  ..
  Provider: null as any,
  Consumer: null as any,
};
..
if (__DEV__) {
  context._currentRenderer = null;
  context._currentRenderer2 = null;
}
```

Flow types them `_currentRenderer?: Object | null`: optional, and `null`
once set. The port's `#[rust_js::nullable]` `Cell<Option<..>>` fields had
one answer for a `None`, `null`, so the literal wrote them, `_currentRenderer:
null`, and DEV set them again: twice, and present in production, where
upstream's object hasn't them.

A plain `Option` field's `None` is left out where its struct is made
already (ADR 0280), but it's `undefined` where it's set, where React writes
`null`. The field is both: optional, `?`, and nullable, `| null`.

The user's decision: an optional field, absent.

## Decision

**`#[rust_js::optional]` on a `#[rust_js::nullable]` field says it's
absent until it's set, Flow's and TypeScript's `field?: T | null`.** Where
its struct is made, its `None` is left out, as any `Option` field's is
(ADR 0280); where it's set, its cell's `set`, `replace` or `take`, or
assigned, `context.label = None`, its `None` is `null`, as a nullable
field's is (ADR 0772). The two attributes are Flow's two marks, each
meaning what it does alone: `nullable` the `| null`, `optional` the `?`.

**A nullable field assigned is written as its cell's `set` writes it**,
every nullable field's, optional or not: `None` is `null`, an `Option`
that may be `None` `label ?? null`, and a `Some(..)`, or what's `null` or
a value already, as it is. It was `context.label = undefined`: ADR 0772
wrote `null` where a cell is set, not where a field is assigned through a
`&mut`, so "`null` where it's set" held of one way to set it and not the
other.

```rust
pub struct ReactContext<T: 'static> {
    ..
    #[cfg_attr(rust_js, rust_js::name = "_currentRenderer")]
    #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
    pub current_renderer: Cell<Option<&'static Unknown>>,
}

let context = ReactContext { .., current_renderer: Cell::new(None), .. };
if __DEV__ {
    context.current_renderer.set(None);
}
```

```js
const context = { $$typeof: REACT_CONTEXT_TYPE, .., Provider: null, Consumer: null };
if (__DEV__) {
  context._currentRenderer = null;
}
```

- Made with an `Option` that may be `None`, it's given it as it is,
  `{ label }`, `undefined` for `None`, as a plain `Option` field is; a
  `Some(x)` is `x`. (Amended below: `null` for `None`, `{ label: label
  ?? null }`.)
- Its value is `null` or a value only once it's set, so it isn't one a
  nullable field or parameter takes as it is: given one, it's `?? null`,
  `{ name: context.label ?? null }` (`bindings::null_where_made`, the one
  answer of whether a field's `None` is `null` from where it's made on,
  which making, `nulled` and ADR 0781's `null_nones` ask).
- Read, absent, `null` or set, it's the `Option` it holds: `is_none()` and
  `match` test `== null`, and `==` of two treats `undefined` and `null`
  alike, as every `Option` read does (ADR 0030).
- Its `.d.ts` is a nullable field's, `field?: T | null | undefined`: one a
  caller may leave out takes `undefined` too (ADR 0196).
- **`#[rust_js::optional]` on a field that isn't nullable is a compile
  error**, as an attribute that does nothing is (ADR 0586): a plain
  `Option` field's `None` is left out already.

  ```
  rust-js: `#[rust_js::optional]` is of a `#[rust_js::nullable]` field, whose `None` is then left out where its struct is made and `null` where it's set; an `Option` field's `None` is left out already
  ```

## Why

- **Upstream's object**: no DEV field in production, `null` in DEV, each
  written once.
- **The marks people know**: Flow's and TypeScript's `?` and `| null`,
  one attribute each; a field with neither is a plain `Option`'s, with
  `nullable` alone always there, `null` or a value.
- **One owner**: whether a field's `None` is `null` where it's made is
  `null_where_made`'s; its writes, a cell's and an assignment's alike,
  `is_nullable`'s, each through `js::Storage::NullForNone.written` (ADR 0791).

## Alternatives

- **`#[rust_js::nullable(optional)]`**: a function's `nullable(..)` names
  its parameters; on a field the same form would name a mode. Rejected.
- **`optional` alone meaning nullable too**: its name says nothing of
  `null`, and a reader of Flow's `?: T | null` would look for both marks.
- **Every nullable field left out where it's made `None`**: react.dev's
  props to Next.js, `errorMessage: null` (ADR 0275), and React's own
  `Provider: null`, `owner: null` (ADR 0781), are there with `null`.

## Consequences

- A comment before an optional field made `None` in the literal goes with
  the field, which isn't written, as one before any field left out does
  (ADR 0280).

## What holds it

- `test/lowering-nullable.test.ts`, "an optional nullable field is left
  out where it's made and null where it's set": made with `None` and set
  in DEV, made with an `Option`, a `None` variable, given to a nullable
  field, assigned `None`, an `Option` and a `Some`, its `.d.ts`, and its
  reads, `is_some`, `is_none`, `==` and `match`, absent, `null` and set,
  each run.
- `test/diagnostics.test.ts`, "an optional field that isn't nullable".
- Mutations `optional-field-made-null`, `optional-unnullable-unrefused`
  (`bindings.rs`), `optional-field-made-null-in-literal`
  (`aggregates.rs`), `optional-field-read-nulled` (`mir/rvalues.rs`),
  `null-none-given-optional` (`mir/patterns.rs`),
  `nullable-assigned-undefined`, `nullable-assigned-some-coalesced`
  (`mir/assign.rs`).

## Amendment: an optional field of what has drop glue is left out too

React's `ReactSharedInternalsClient`, a library, made its DEV-only
`actQueue: Cell<Option<Vec<RendererTask>>>`, each task a `Box<dyn
FnMut>`, `actQueue: {}.value`: drop glue gave its `Cell::new(None)` a
temporary of its own, a drop flag with it, and the struct's field read
that temporary's `value`, which tidy then read in place, past where the
cell made there would have been read as what it holds.

**A cell made in its place and read is what it holds**, `{ value: x
}.value` `x`, wherever tidy moves a temporary's value to be read
(`moves.rs`), as `Expr::member` reads one where it's made; then the
field's `undefined` is no key (ADR 0280). Not an assignment's target,
which is no read. Case N.

**Its `set(None)` drops what it held, as Rust does**: of a library, a
consumer may make a `RendererTask` of a closure that holds what has a
destructor, which `$dispose` runs (ADR 0521), so the drop stays, `const
old = ..actQueue; ..actQueue = null; if (old != null) { for (const item
of old) { $dispose(item); } }`. Of a crate no closure of which with a
destructor is made a `dyn Fn`, a binary's, it's `actQueue = null;` alone.
Case C: results change where a destructor runs. Knowing the field is
still `None` where the module's `on_load!` sets it, as it is in React's,
would drop nothing there: not yet.

- `test/lowering-nullable.test.ts`, "an optional nullable field of what
  has drop glue is left out where it's made".
- Mutations `made-cell-read-unfolded`, `made-cell-target-folded`
  (`tidy/moves.rs`).

## Amendment: made with what may be `None`, it's `null` for `None`

React's `ReactElement` makes its object's `ref` of what `props.ref` is,
`null` where it isn't there:

```js
const ref = refProp !== undefined ? refProp : null;
..
element = { $$typeof: REACT_ELEMENT_TYPE, type, key, ref, props };
```

The port's `let r#ref = if !js::strict_equals(ref_prop, None::<&Unknown>)
{ ref_prop } else { None };` of a `#[rust_js::nullable, rust_js::optional]`
field printed `const ref = refProp !== undefined ? refProp : undefined;
return { ref };`: the key there, and `undefined`. That's neither of what
the field is to JS, `ref?: T | null`, absent or `null` or a value, which
`Object.hasOwn`, `Object.keys`, a spread and Jest's `toEqual(null)` each
tell apart. The rule above, "given an `Option` that may be `None`, it's
given it as it is", wrote it.

**An optional nullable field is left out only where it's `None` and
nothing else**: the literal's own `None`, `S { label: None }`, or a
`Cell::new(None)`, through MIR's temporary of either too
(`written_none`, `aggregates::Given::WrittenNone`). **Made with anything
else, a variable, a parameter or a call that may give `None`, it's stored
as any nullable field is**, `js::Storage::NullForNone.written`: `{ label:
label ?? null }`, what's never `undefined` as it is, and a variable only
nullable fields read is `null` where it's made, `const label = null;`
(ADR 0781's `null_nones`, which now counts an optional field's read
too; the literal's own `None` is left out whatever its temporary holds). Its key is
there wherever the literal writes one but a `None`, as a JS literal's
`{ ref }` is.

```rust
let title = if n > 1 { Some(n.to_string()) } else { None };
Titled { title }
```

```js
const title = n > 1 ? String(n) : null;
return { title };
```

Case ([0262](0262-when-rust-and-js-disagree.md)): A for the program, which
reads `undefined`, `null` and an absent key alike as `None`; to JS, the
field is now what its declaration says, `field?: T | null`, never there
and `undefined`.

- What it costs: an `Option` that may be `undefined`, React's `owner`
  parameter, is `_owner: owner ?? null` where the field takes it, where
  upstream's `_owner: owner` relies on its callers giving `null`.
- A read of it is still not `null` or a value (`null_where_made`, which
  `nulled` asks): it may be absent.
- Upstream's `refProp !== undefined ? refProp : null` with `{ ref }` needs
  a fact rust-js hasn't, that the branch of `!js::strict_equals(x, None)`
  reads an `x` that isn't `undefined`; without it, the port's prints
  `const ref = refProp !== undefined ? refProp : undefined; return { ref:
  ref ?? null };`, right but a `?? null` longer.

What holds it:

- `test/lowering-nullable.test.ts`, "an optional nullable field is left
  out where it's made and null where it's set": a parameter, a `None`
  variable, copied and moved, React's `ref`, a `Some` or `None` variable
  and one written in the literal, each run, and none of their objects
  holding `undefined`.
- Mutations `optional-field-made-null-in-literal`,
  `optional-written-none-unasked`, `optional-field-given-undefined`
  (`aggregates.rs`), `null-none-given-optional`,
  `written-none-of-two-writes`,
  `written-none-cell-unasked` (`mir/patterns.rs`).

## Amendment: given another optional field's value, it's as it is

Case ([0262](0262-when-rust-and-js-disagree.md)): C. React's
`processTransitionCallbacks` copies each abort's `name?: string | null`
into the deletion it hands `onMarkerIncomplete`:

```js
filteredAborts.push({ type: 'marker', name: abort.name, endTime });
```

The port's `TransitionAbort` and `TransitionTracingDeletion` both have
`#[rust_js::nullable, rust_js::optional] name: Option<String>`, and
`name: abort.name.clone()` printed `name: abort.name ?? null`: an abort
without a name gave the callback `null` where upstream gives it
`undefined`, which a JS callback can tell (`=== undefined`, `toEqual`).
The rule above, "a read of it isn't one a nullable field takes as it is",
was written for a nullable field that isn't optional, which holds `null`
or a value; an optional one holds what's absent too.

**An optional nullable field given what another optional nullable field
holds, read, stores it as it is**: absent stays `undefined`, `null`
stays `null`, a value stays the value. Where its struct is made, `{
name: abort.name }`, and where it's assigned, `deletion.name =
abort.name`, through MIR's temporary, a `Copy`'s read or a `clone()` of
a string, a number or a reference, as `nulled` follows a nullable
field's (`stored_given`'s `Given::Optional`). A nullable field that
isn't optional is still given it `?? null`, as it holds `null` or a
value. Its `.d.ts`, `name?: string | null | undefined`, says so already.

- `test/lowering-optional-fields.test.ts`, "an optional nullable field
  given another's value stores it as it is": objects JS makes, the key
  absent, `null` and a string, copied where a struct is made and where
  it's assigned, and given to a nullable field that isn't optional.
- Mutations `optional-read-coalesced` (`mir/rvalues.rs`),
  `optional-assigned-coalesced` (`mir/assign.rs`),
  `optional-given-optional-coalesced` and `optional-given-any-nullable`
  (`aggregates.rs`).
