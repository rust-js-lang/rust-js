# 1071. A flattened enum is told apart by its keys

Status: Accepted (the user's decision on #504, 2026-11-03: flatten, and
tag by key). Extends [0204](0204-flattened-props.md), whose flattened
field is a struct of named fields, and
[1044](1044-a-flattened-state-is-its-classs-own-keys.md), whose is a
`RefCell` of a discriminated union, to an enum with no tag, and to a type
parameter given one.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, no observable
difference. Each value of the enum the program makes has its variant's
keys, even where a key holds `undefined`, and no other variant's, so
`'getter' in node` tells the variant as Rust's discriminant does. A whole
write deletes the keys of every other variant first.

## Context

alien-signals' nodes are TypeScript interfaces that extend one,
`interface ComputedNode extends ReactiveNode { value; getter }`. So each
node is one flat object, `{ value: undefined, subs: undefined, ..,
flags: 0, getter }`, and the code tells nodes apart by a member:
`if ('getter' in node)`, `else if ('currentValue' in node)`,
`!('getter' in dep) && !('currentValue' in dep)`.

Its generic graph module (system.ts) can't know the kinds, so the Rust
an engineer writes is a generic node of its caller's kind, and an enum
of the kinds:

```rust
pub struct ReactiveNode<K> {
    pub deps: RefCell<Option<Rc<Link<K>>>>,
    // depsTail, subs, subsTail
    pub flags: Cell<i32>,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub kind: K,
}

pub enum NodeKind {
    Signal { current_value: Cell<Value>, pending_value: Cell<Value> },
    Computed {
        value: Cell<Value>,
        #[cfg_attr(rust_js, rust_js::test)]
        getter: Box<dyn Fn(Value) -> Value>,
    },
    Effect { r#fn: Box<dyn Fn() -> Option<Rc<dyn Fn()>>>, cleanup: RefCell<..> },
    EffectScope,
}
```

Without the attribute, the kind is an object nested in the node, and
its tests read its tag: `node.kind.TAG === "Computed"`,
`c.kind.value = c.kind.getter(oldValue)`.

## Decision

**An enum that a `#[rust_js::flatten]` field holds is told apart by its
keys where each variant of named fields has a key that no other variant
has, and at most one variant has no fields** (`bindings::keyed_shape`).
The key that tells a variant is the field `#[rust_js::test]` marks;
without one, it's the variant's first key that no other variant has.
This is the same attribute that marks the member telling an untagged
enum's struct apart (ADR 1036). `bindings::flattened` finds the crate's
keyed enums once, in two places. The first is a field that names the enum,
`kind: NodeKind`. The second is a type that gives the enum to a struct's
flattened type parameter, `ReactiveNode<NodeKind>` of `kind: K`. Either
can appear in a body's expression or an item's signature.

- **Every value of the enum is its keys, with no tag**, wherever it's
  made, whether it's flattened or not. `Signal { current_value,
  pending_value }` is `{ currentValue, pendingValue }`, and the variant
  without fields is `{}`. A rustc constant of one is the same. **Each key
  is present even where it holds `undefined`** (the object's `kept` keys,
  ADR 0280), so `in` finds it.
- **A variant's test is its key**: `"getter" in node`. The variant
  without fields is told by none of the others' keys, and so is an
  or-pattern that includes it: `matches!(dep.kind, Effect { .. } |
  EffectScope)` is `!("currentValue" in dep) && !("getter" in dep)`, as
  upstream writes it.
- **Flattened, the keys are the parent's own.** A variant made in place
  is written into the parent's literal. Any other value is spread there,
  `{ flags, ...kind }`. The field read through, `&node.kind`, is the
  parent object (ADR 0204, amended), so `c.value = c.getter(oldValue)`.
- **A whole write deletes the other variants' keys.** `node.kind =
  Computed { .. }` computes the new fields first, as Rust does, then
  deletes every key the new variant doesn't have, then sets its own:
  `delete node.currentValue; ..; node.value = value; node.getter =
  double;`. A value that no literal spells out deletes every key and is
  given with `Object.assign(node, kind)`. This is the runtime variant
  change; without the deletes, an old key would still tell the old
  variant.
- **A type parameter may be flattened**, `#[rust_js::flatten] kind: K`.
  Generic code builds a node with the kind's keys spread into it. A whole
  write, or a move out, of generic code is refused, because the code
  can't know which keys to delete.
- **Declared,** the enum is the union of its variants' objects, `{}` for
  the one without fields: `export type NodeKind = { currentValue: ..;
  pendingValue: .. } | { value: ..; getter: .. } | { fn: ..; cleanup: .. }
  | {};`. A struct that flattens such an enum, or a type parameter, is
  its own members and that type, `export type ReactiveNode<K> = { deps:
  ..; flags: number } & K;`. An interface can extend neither. So
  `ReactiveNode<NodeKind>` is upstream's interfaces that extend
  `ReactiveNode`, taken together.
- **Another crate** reads which of a library's enums are keyed from the
  library's contract (`keyed`, library ABI 15). A consumer represents
  those enums as the library does, and gives them to a flattened type
  parameter of the library's structs.

Refused:

- a flattened enum that no key tells apart, either because it has two
  variants without fields or because a variant's every key is another
  variant's too;
- a key that the parent also has as a field of its own;
- a flattened type parameter given anything but such an enum, or another
  type parameter;
- the field moved out, or lent `&mut` whole, since it's no object of its
  own.

**One way.** `#[rust_js::flatten]` stays the only way to ask that a
field's value be its parent's own keys. Whether the enum is told by its
keys is derived from that attribute, with no attribute on the enum. One
owner, `bindings::flattened`, the extended `flattened_states`, answers it
for representation, validation, construction, writes and declarations.

## Why

- It's upstream's object and upstream's tests. The Rust is the enum an
  engineer writes, read by `match`, and the generic graph knows nothing
  of it.
- Keys present even when `undefined` are what make `in` a sound test. That
  was ADR 1036's reason not to test a member's presence, since a struct's
  `None` field is left out.

## Alternatives

- **`#[rust_js::untagged]` on the enum**, as serde writes `#[serde(untagged)]`
  beside `#[serde(flatten)]`. It's intrinsic to the enum, so no contract
  fact would be needed. But it's a second attribute for what the
  flattening already says. The user decided to derive it from the
  flattened field.
- **A `TAG` key in the parent** (ADR 1044's form, of a `RefCell`): a key
  upstream doesn't have.
- **No deletes on a whole write**, as 1044's state keeps an old
  variant's keys. That's sound there, because its tag tells the variant.
  Here an old key would still tell the old variant, which is a wrong
  answer.

## Consequences

- alien-signals' index.js tests `'getter' in node` and reads `c.value`,
  as upstream does, and its suite passes 202 of 202.
- Held by the corpus's `flattened_keyed_enum` (each variant made in
  place and given, matched, lent, a variant holding `None` told, and each
  kind of whole write at runtime, one reading the old variant),
  `flattened_keyed_param` (a generic graph's node of the caller's kind)
  and `flattened_keyed_const` (constants), each against native Rust; and
  `test/flattened-keyed.test.ts` (declarations, each refusal, and a
  library's keyed enum in its consumer, run). The mutations are in
  `bindings/flattened.ts` and `mir/keyed_writes.ts`, and these are
  elsewhere: `keyed-shadow-unchecked`, `keyed-param-given-anything`,
  `keyed-placed-refused` and `keyed-placed-anything` (`analysis/validation.ts`);
  `keyed-otherwise-of-all` and `keyed-unit-a-name` (`representation.ts`);
  `keyed-const-unit-a-name` and `keyed-const-keys-dropped`
  (`representation/consts.ts`); `keyed-tag-kept`, `keyed-keys-dropped`,
  `flattened-inner-kept-lost` and `const-object-kept-lost` (`aggregates.ts`);
  `keyed-write-unseen` (`mir/assign.ts`); `keyed-declared-tagged` and
  `flattened-param-extended` (`declarations.ts`); and
  `keyed-contract-unwritten` (`library.ts`).
- `representation.rs`'s writer of rustc's values, `const_js`, moved to
  `representation/consts.rs` with its mutations, to keep the file under
  the cap.
