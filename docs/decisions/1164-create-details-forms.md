# 1164. The forms upstream's createDetails needs

Status: Accepted (2026-11-15, the lead's decisions on Codex's contract,
rust-js-lang/vercel-ui-sdk-in-rust@2680f613e,
docs/create-details-contract.md). Each form is a section, landed as it's
built.

Case ([0262](0262-when-rust-and-js-disagree.md)): none for each form's
JS, which is what it was; the declarations change, to say what it is.

## Context

The vercel UI kernel's `kernel/details.ts` (vercel-labs/ui@220c1a0) is
the details every `on<Key>Change` callback receives: a vocabulary of
reasons, `ReasonVocabulary`, each with the DOM event behind it; the
details of a change, `ChangeDetails<Reason>`, one member per reason so
checking `reason` narrows `event`; and `createDetails<const Given,
Cancelable extends boolean = true>(given, source, cancelable)`, whose
result keeps `given`'s own keys, then `source`, then a detachable
`cancel` and a live `get canceled()`. Codex's contract asks for its
declarations and ABI as they are. The lead approved seven forms (A to G)
and rejected a default-parameter print (F as first proposed) and a
flatten of a reference (G as first proposed); this records each.

## A. A trait's impls as one type map

**A unit struct named as a string is that string, its type as its value
is**: `#[cfg_attr(rust_js, rust_js::name = "trigger-press")] pub struct
TriggerPress;` is `"trigger-press"` in JS (ADR 0013), and now `export
type TriggerPress = "trigger-press";`, so a function giving one is typed
`TriggerPress`, no longer `any`. Its `rust_js::name` is its value, not
its type's name, but for a class's or a namespace's (`named_unit`,
`type_name`).

**`#[cfg_attr(rust_js, rust_js::type_map(map = "ReasonVocabulary", keys =
"ChangeReason", lookup = "EventOf"))]` on a trait of one associated type**
declares its impls for such structs as one object type, its keys and a
lookup of it (`declarations/type_maps.rs`):

```ts
export type ReasonVocabulary = {
    /** Click, tap, Enter or Space on the part that opens. */
    "trigger-press": PressEvent;
    /** Escape. */
    "escape-key": KeyboardEvent;
};
export type ChangeReason = keyof ReasonVocabulary;
export type EventOf<Reason extends string> = Reason extends ChangeReason ? ReasonVocabulary[Reason] : Event;
```

- Each key is an impl's struct's name, its value the impl's associated
  type, in the order the source writes the impls, with the struct's doc
  comment, which a caller's editor shows.
- The lookup's other strings take the value of the trait's impl for
  `&'static str`, an open reason, upstream's `: Event`; without one,
  `never`. Its type parameter is the trait's name, as upstream's is.
- The impls are the one source of all three: no second list of reasons.
- Refused, saying what to write: a `type_map` without `map`, a `lookup`
  without `keys`, a trait of other than one associated type, and an impl
  of a type that's neither a named unit struct nor `&'static str`.
- The printer gains a conditional type, and a type alias of an object
  prints its members' doc comments, as an interface's do.

## C. A struct declared of each of a union's members

**`#[cfg_attr(rust_js, rust_js::distributive)]` on a generic struct of one
type parameter** declares it a distributive conditional type,
`CommitDetails<R extends string = string> = R extends unknown ? { reason:
R; event?: EventOf<R> | undefined; .. } : never`, so a TypeScript caller's
check of `reason` narrows `event`, as upstream's `ChangeDetails` and
`CommitDetails` are one member per reason. Of another number of type
parameters, or flattening a struct its declaration would extend, it's
refused.

- **A type parameter's default is TypeScript's**: `R: Reason = &'static
  str` is `R extends string = string`, so `CommitDetails` without its
  reason takes any. Every struct's default is declared now, where it was
  left out; the printer passes TypeScript's native factory its default in
  the right place.
- **A type map's associated type is its lookup**: `<R as Reason>::Event`
  is `EventOf<R>`, or without a lookup `ReasonVocabulary[R]`, where it was
  `any`, so the field's event is the reason's.
- A distributive alias's members carry their doc comments, as an
  interface's do.

## D. A type parameter's `const` and its default

**`#[cfg_attr(rust_js, rust_js::const_param)]` on a generic parameter**
is TypeScript's `const` one, whose caller's literals are kept as
literals, upstream's `const Given`: `kept<const G>(given: G): G`, so
`kept({ reason: "escape-key" }).reason` is `"escape-key"`, not `string`.
It's declaration-only: it names nothing and changes no JS.

**A function's type parameter's default is its bound `js::DefaultsTo<T>`'s
argument**: `C: js::Boolean + js::DefaultsTo<js::True>` is `C extends
boolean = true`, where Rust has no default for a function's type
parameter. `js::DefaultsTo<T>` is a marker every type meets, so it
constrains nothing in Rust, and its argument is a type rustc resolved
and checked; no path in an attribute is resolved by name (the lead's
decision, against a path written in `rust_js::default_type`). A struct's
own default, `R = &'static str`, is declared as C says.

`#[rust_js::types]`'s reader reads a conditional type, `C extends true ?
"change" : "commit"`, its branches types themselves.

## E. Literal booleans

**`js::True` and `js::False` are unit structs whose values are `true` and
`false`**, `#[rust_js::name = true]`, a fieldless variant's form (ADR
0284) extended to unit structs (`UnitValue`), declared `true` and `false`
where a function gives one. **`js::Boolean`**, sealed, is `bool`, `True`
and `False`, TypeScript's `boolean`; **`js::boolean(value)`** is the
value itself as a `bool`, an identity binding as `js::unknown` is, so
`!js::boolean(cancelable)` is `!cancelable` and generic code over a `C:
js::Boolean` takes no dictionary: upstream's `if (!cancelable)`, with no
argument the JS ABI lacks. Native Rust runs `js::boolean` too, by the
sealed trait. No js crate conversion did this before: `js::truthy` asks
JS's truthiness of any value, another question.

## B. A live accessor

**`js::Accessor<T>`, a field of a struct a literal makes, is an own,
enumerable, configurable getter without a setter**, `get canceled() {
return canceled; }`, of the function it's made of, `js::Accessor::new(move
|| canceled.get())`, so each read is what it gives then, upstream's `get
canceled()`. **`details.canceled.get()` is `details.canceled`**, the
property's read; a spread of the details, `{ ...details }`, has the value
it gives then, as JS's has. It's declared `readonly canceled: boolean`.
Native Rust holds the function and calls it.

- `js::Accessor::new` and `get` are identity bindings, the value itself:
  the getter's function, and the property's read where `get` is given its
  field.
- `adt_value` makes the field a `Prop::Getter`, which every later pass
  knows is no data property (`accessors_got`).
- Refused, saying what to write (`analysis/accessors.rs`): the field moved
  or copied out of its struct, `let Details { canceled } = details`, which
  JS would read the value of there, not where Rust's `get` calls it; and
  `get` of an accessor that isn't a struct's field, a local's, whose JS is
  its function, not what it gives.
- What's left: the getter's doc comment isn't printed yet, where a
  method's is.

## G. A struct that begins with a copy of a value's keys

**A flattened field of `js::Spread<&'static G>`, `js::Spread::new(given)`,
is JS's own spread of the value where the struct is made**, `{ ...given,
source }`, upstream's: its own enumerable keys, symbols too, each getter
and Proxy trap run as the literal runs them, an own `undefined` kept, what
isn't own or enumerable left out, the struct's own keys after it winning,
and the value never written. It's declared `{ source: string } & Omit<G,
"source">`. Rust reads the copy only as that declares: `js::Spread` has no
deref to `T`, so nothing reads a later write through the source as the
copy's. That keeps #1822's refusal of a flattened reference, whose
parent's copy would miss a write Rust's held reference sees (the lead's
decision, against a flattened `&'static G`).

- **One copy, two placements**: `js::object::spread(value)` is the copy as
  a value alone, and of a tuple `{ ...a, ...b }`; the flattened field is
  the same copy at the start of a literal with keys after it. Each writes
  JS's spread, one node of the JS tree, `Prop::Spread`. A struct whose
  only field is a `js::Spread` is refused, naming `js::object::spread`, so
  each case has one form; and a `js::Spread` field without `flatten` is
  refused, saying to flatten it.
- A spread of a struct holding the accessor wouldn't do: `{ ...{ get
  canceled() {..} } }` copies the value, not the getter, so the literal
  is the struct's own.
- An intersection alias's object prints its members' doc comments.

## Consequences

- test/declarations-type-maps.test.ts: the declarations, a strict
  TypeScript caller (literal reasons, the lookup of a known and an
  unknown reason, the negatives), and the refusals;
  test/typescript.test.ts's conditional type and object alias's docs.
- Mutations trait-type-map-undeclared, named-unit-type-its-value,
  named-unit-not-declared (`declarations.rs`),
  type-map-lookup-of-others-never, type-map-keys-unwritten,
  type-map-members-undocumented (`declarations/type_maps.rs`),
  object-alias-members-undocumented, conditional-type-unprinted
  (`typescript/index.js`); transparent-tuple-struct-any and
  type-name-given-unread retargeted.
- C: declarations-type-maps' `CommitDetails` (its declaration, a strict
  caller whose `reason` narrows `event`, the default, a wrong event
  refused); typescript.test's default and distributive alias; mutations
  distributive-declared-as-interface (`structures.rs`),
  struct-type-default-undeclared, type-map-projection-any
  (`declarations.rs`), type-parameter-default-unprinted,
  distributive-members-undocumented (`typescript/index.js`);
  object-alias-members-undocumented retargeted.
- D and E: declarations-type-maps' literal booleans (`kept<const G>`,
  `pick<C extends boolean = true>`, `yes(): true`, a written conditional,
  a strict caller, and the JS run); typescript.test's `const` parameter;
  mutations const-param-unread, defaults-to-unread, js-literal-boolean-any
  (`declarations.rs`), unit-struct-boolean-unread (`bindings.rs`),
  written-conditional-unread (`written.rs`), const-type-parameter-unprinted
  (`typescript/index.js`); unit-struct-field-always-there and
  named-unit-const-undefined retargeted.
- B: test/js-accessor.test.ts (the descriptor, the live read, a detached
  `cancel`, a spread's snapshot, the declaration, the two refusals);
  mutations accessor-field-a-function (`aggregates.rs`),
  accessor-field-writable (`structures.rs`), accessor-get-of-a-local-allowed,
  accessor-field-moved-out-allowed (`analysis/accessors.rs`).
- G: test/js-spread.test.ts (the JS, the declaration, the trace of the
  same Proxy and getter against upstream's literal, symbols, an own
  `undefined`, a non-enumerable key, the struct's key winning, a strict
  caller, the refusals); mutations spread-field-refused,
  spread-unflattened-allowed, spread-alone-allowed (`validation.rs`),
  spread-declared-any-keys, spread-omits-nothing (`structures.rs`),
  intersection-members-undocumented (`typescript/index.js`);
  untagged-records-declared-extended, flattened-param-extended and
  distributive-members-undocumented retargeted.
- Guide: interop/export-to-javascript.md, TypeScript declarations;
  interop/types.md, `js::object::spread`.

## Amendment: type maps describe declarations without rooting dictionaries

A declaration type map and a typed trait that contributes no items of its
own emit implementation dictionaries only when the compiler's reachability
graph needs them (#2090). An erased value or generic call that reads a
dictionary still reaches it. A type map member reads its associated type as
written, and a record keeps a type-map key as a type parameter even when only
one field uses it. A written generic default likewise keeps its alias.
