# 0204. A flattened field's struct is its parent's props

Status: Accepted. Extends [0195](0195-rest-props.md) and [0196](0196-typescript-declarations.md);
extended by [1044](1044-a-flattened-state-is-its-classs-own-keys.md): a `RefCell` of a
discriminated union is its class's own keys; by
[1071](1071-a-flattened-enum-is-told-by-its-keys.md): an enum told apart
by its keys, or a type parameter given one;
amended by [0205](0205-flattened-chains.md), which chains them and gives a
name both have to the props; and by
[0674](0674-a-struct-literals-fields-are-in-the-order-written.md): a flattened
field made outside JSX is found by its name, where it's written. Amended
below: a flattened field lent whole is its parent's object; and a flattened
reference is refused wherever its parent's copy could be told from its
referent (#1819); and a JS object is
never flattened (#1822), superseding "a flattened field may be a JS
object too"; and a struct lent whole, or read whole by
value, is its own fields wherever it's copied, compared or hashed (#1820).

> **Note:** since [ADR 0438](0438-an-optional-prop-is-given-some.md), a
> left-out optional prop is `None`, so the `{..Default::default()}` in this
> decision's examples is no longer needed; it's written only to give a base
> of one's own. The current form is in the guide's
> [Optional props](../guide/react/components.md#optional-props).

Case: N, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

TypeScript makes a component's props of other types', `AnchorProps &
ButtonLinkProps`: one object, every field of both, and `...props` what
the component doesn't take by name. Rust has no `&`: a struct names its
fields, and one holding another holds it as a field, nested, where JS's
props are flat. What a component takes besides was a `Rest` (ADR 0195),
an object Rust can't look into.

## Decision

**A props struct's field marked `#[rust_js::flatten]` holds a struct whose
fields are the component's own props, as serde's `#[serde(flatten)]` has
a struct's fields in its parent's JSON. It's a `Rest`, typed: what the
component takes apart besides its other fields.**

```rust
#[derive(Default)]
pub struct Anchor {
    pub href: Option<&'static str>,
    pub target: Option<&'static str>,
}

#[derive(Default)]
pub struct ButtonLinkProps<C> {
    pub size: Option<&'static str>,
    pub children: C,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub anchor: Anchor,
}

pub fn ButtonLink<C: Node>(ButtonLinkProps { size, children, anchor }: ButtonLinkProps<C>) -> Element { .. }

<ButtonLink size={Some("lg")} anchor={Anchor { href: Some("/learn"), ..Default::default() }} {..Default::default()}>
```

```jsx
export function ButtonLink({ size, children, ...anchor }) { .. }

<ButtonLink size="lg" href="/learn">
```

```ts
export interface ButtonLinkProps<C> extends Anchor {
  size?: string;
  children: C;
}
```

- **Taken apart, it's `...anchor`**, its fields read as any struct's,
  `anchor.target`, and spread as any struct is, `<a {...anchor}>`.
- **In JSX, one made there is its fields, each an attribute**; another,
  `{...anchor}`.
- **Declared, it's what the props extend**: its struct, or, one of
  another module, which TypeScript there can't see (ADR 0196), what a
  `Rest` is, `[prop: string]: unknown`.
- **What JS's props can't be is an error**: a struct with two rests, a
  `Rest` and a flattened field or two of these; a flattened field that
  isn't a struct of named fields, or is one within another; a field name
  both have; the field read, `p.anchor`, or taken apart anywhere but
  where the props are given; and a struct with one made anywhere but as
  `jsx!`'s props, or their `{..Default::default()}`, as it would be
  nested.

## Why

- **It's the props JS has**: one flat object, which TypeScript types as
  the two together, and a JS caller gives, `<ButtonLink href="/x">`.
- **It's Rust's own struct**, typed: each field is checked where it's
  given and read, where a `Rest`'s aren't.
- **It's tested**: a JSX test renders a component with a flattened
  struct, called by Rust and by `createElement`, and refuses each misuse;
  the declarations test checks TypeScript sees a flattened struct's
  fields as the component's own, and finds a wrong one.

## Costs

- **A Rust caller writes the struct**, `anchor={Anchor { .. }}`: `jsx!`
  doesn't know a component's fields, so it can't take `href` as one.
- **What a JS caller gives besides is in it too**, as TypeScript's `&`
  allows: `{...anchor}` passes it on.
- **No `Omit`**: a name both structs have is an error, so a flattened
  struct holds only what its parent doesn't. ADR 0205 gives it to the
  parent.

## Amendment: made outside JSX

A struct with a flattened field made outside JSX is one object too: its
own fields and the flattened one's, a struct made there written in place
and another value spread where the field is declared, `{ ...item,
severity }`, as react.dev's runESLint writes `{...item, severity:
severity[item.severity]}`. A derived `Default`'s is too. A flattened field
may be a reference to a struct, read through as the struct is,
`report.item.line` being `report.line`. They were refused, made only as
JSX's.

A struct of more than one rest, two flattened fields, is made as two
spreads (2026-10-10), `{ ...template, ...files }`, the later's keys over
the earlier's, as react.dev's SandpackRoot gives Sandpack
`{...template, ...files}`. JS's object has their keys mixed, so such a
struct is neither taken apart, which would be two rests, nor read
through one of them, whose keys another may have; each is refused. A
flattened field may be a JS object too, a `Dict` whose keys spread
(superseded below, #1822: a JS object is never flattened). A
struct of more than one rest was refused whole. A JSX test makes one of
a struct and one of a struct and a `Dict`, and refuses the read and the
taking apart; mutations take a struct of one rest as one of two, allow
each refused one, and refuse a `Dict`.

An enum variant's flattened field (2026-10-10) is refused, at the field:
`#[rust_js::flatten]` there was read nowhere, so a tagged variant of a
shared struct was made nested, `{ type: "article", base: { title } }`,
where JS holds it flat, as Next.js's `OpenGraph` is. Making one flat, and
matching it, is a variant's own work, not done yet. A lowering test
refuses one; a mutation allows it.

## Amendment: lent whole, it's its parent's object (2026-11-01)

React DOM's legacy Fizz config's render state extends the Fizz config's:
upstream copies each of its fields into a new object with one more,
`generateStaticMarkup`, and gives that object to the Fizz config's own
functions, `pushTextInstanceImpl(target, text, renderState, ..)`, which
read the fields they know. Its port flattens the base into it:

```rust
pub struct RenderState {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub base: base::RenderState,
    pub generate_static_markup: bool,
}

push_text_instance_impl(target, text, &render_state.base, text_embedded)
```

A flattened field read whole was an error, as JS's one object has no
field that holds it. **Lent whole, `&state.base`, it's its parent's
object** (`validation.rs`'s `reject_flatten_misuse`): the object has its
fields, each where a read of it is, so what reads or sets them through
the reference reads and sets the parent's, as Rust's do the field's.
`pushTextInstanceImpl(target, text, renderState, textEmbedded)`.

What would read more than its fields is refused:

- **Of a type with `Clone`, `PartialEq`, `PartialOrd` or `Hash`**, whose
  JS reads its object whole: a clone is `{ ...state }`, which would copy
  the parent's fields too, and `==` compares every key.
- **With a name its parent has**, or one a struct its parent is flattened
  in has: JS's one object holds the parent's under it (ADR 0205).
- **Lent mutably**, `&mut state.base`, which may replace it whole; and
  read by value, as before.

Case ([0262](0262-when-rust-and-js-disagree.md)): A. Where it's allowed,
the object has every field of the struct, with Rust's values, and what's
read of it is only those. JS given it, a binding's parameter typed as the
base, sees the parent's fields too, as a JS object that extends another
is seen; ADR 0204's "what a JS caller gives besides is in it too".

Held by the corpus's `flattened_state_lent`, against native Rust;
`test/lowering.test.ts`'s "a flattened field lent whole is refused where
its type reads it whole, or lent mutably", and `test/jsx-props.test.ts`'s
chain lent whole; and mutations `flattened-lent-refused`,
`flattened-lent-mutably`, `flattened-lent-read-whole` and
`flattened-lent-shadow-unseen` (`analysis/validation.rs`).

## Amendment: a reference is a copy, refused where it could be told apart (2026-11-15)

A wrong answer (P0, #1819). A flattened field that's a reference to a
struct, `item: &'a Item`, makes its parent `{ ...item, severity }`: a copy
of the referent's keys. Rust's parent shares the referent, so what changes
it after the parent is made, through either, is seen through both, and
JS's copy sees neither. With a `Cell` in `Item`, `item.hits.set(5)` was
`0` read through `report.item.hits`, and the other way; with a flattened
`&'a mut Item`, `report.item.line = 8` left `item.line` 3.

**A flattened reference is allowed only where nothing could tell its
parent's copy from its referent**, one rule in `bindings::flattened`
(`copied_references`, `copy_seen`, `copy_seen_in_crate`), refused at the
field, naming it, otherwise:

- **A `&mut`**, whose writes through the parent the referent wouldn't see.
- **A referent with a cell among its own keys**, a `Cell` or `RefCell`,
  whose writes through either the other wouldn't see: what rustc's
  `Freeze` says. What it holds behind a pointer, an `Rc<RefCell<..>>`, is
  one object both share, as Rust's are.
- **A referent the crate changes in place or compares the address of**,
  `ptr::eq` or a raw pointer to one: the mutation owner's `Mutated.types`
  (ADR 0407 puts each raw pointer's pointee there), asked after it's
  found (`validation::reject_flattened_copies`), of the crate's structs and
  a library's it names.
- **The reference lent whole**, `&report.item`, which would be the
  parent's object, not its referent. It was refused before by
  `reads_whole`'s answer of any type not an ADT, whose message named
  `Clone` and `PartialEq`; it's refused for what it is now.

Read through, `report.item.line`, it's the parent's key, which holds the
referent's value. A reference to a JS object is the next amendment's.

Case ([0262](0262-when-rust-and-js-disagree.md)): N where it's allowed,
since no program can tell the copy from its referent; D where it could,
a compile error.

Held by the corpus's `flattened_reference_shared` (a referent shared by
two parents, its `Rc<RefCell<Vec>>` written through the source and a
parent, against native Rust) and `test/lowering.test.ts`'s "a flattened
reference is refused wherever its parent's copy could be told from its
referent" (a `Cell` written through the source, a `RefCell` through the
parent, a `&mut` through the parent, `ptr::eq`, a raw pointer, lent
whole, each refused by name; read through alone, compiled); and mutations
`flattened-reference-uncopied`, `flattened-reference-mut-allowed`,
`flattened-reference-cell-allowed` and
`flattened-reference-changed-unasked` (`bindings/flattened.ts`), and
`flattened-reference-lent-whole` (`analysis/validation.ts`).

## Amendment: a JS object is never flattened (2026-11-15)

A wrong answer (P0, #1822), of the family above. A flattened JS object,
`extra: &'a js::Dict<u32>`, made its parent `{ size: 1, ...extra }`, a copy
of its keys. Anything holding a JS object may write it through `&`,
`js::dict::set` or a JS function it's given, so a key set after the parent
is made was in Rust's parent, which holds the object, and not in JS's copy:
`js::has(js::unknown(&props), "k")`, of a parent that's `js::Defined`, was
`false` where Rust's is `true`, and JS given the parent read `p.k` as
`undefined`. rustc calls such a type `Freeze`, so the rule above didn't
see it, and no crate-level fact can show the object unwritten while the
parent lives: a binding, or JS, may write it.

**A JS object is never flattened** (`bindings::copy_seen`, its
`JS_OBJECT_COPIED`): by reference, a `&Dict` or a `&js::Unknown`; by
value, a `Copy` handle, whose copies are one object; or given to a
flattened type parameter (`validation.rs`). The refusal names what to
write: copy it where it's made, `js::object::spread(..)`, the copy JS's
`{ ...files }` is, `js::object::spread((Some(js::unknown(template)),
Some(js::unknown(files))))`, `{ ...template, ...files }`. This supersedes
"a flattened field may be a JS object too" above. Its one known user,
react.dev's SandpackRoot (`files: &'a Dict<SandpackFile>` beside `template:
&'a Template`, in the port at nguyenyou/rust-js-react.dev 6cd15a6), would
write that spread; its result is a `&JsObject`, looser than TypeScript's
intersection, #1825.

Case ([0262](0262-when-rust-and-js-disagree.md)): D, a compile error: the
copy can be told apart, and Rust's sharing can't be kept in one object.

Held by `test/lowering.test.ts`'s "a flattened JS object is refused, by
reference, by value or given to a type parameter" (the key set after the
parent is made, read through `js::has`; a `Copy` handle; a type parameter
given one), `test/lowering-structs-enums.test.ts`'s `&js::Unknown` and
`&js::Dict` rows and `test/jsx-props.test.ts`'s dictionary row, each
refused by name; and mutations `flattened-js-object-allowed`,
`flattened-js-object-by-value-unasked` (`bindings/flattened.ts`) and
`flattened-param-js-object-allowed` (`analysis/validation.ts`).
`test/jsx-props.test.ts`'s spread of spreads (ADR 0364) spreads a clone of
a struct that changes now, where it spread a dictionary that did.

## Amendment: a struct read whole is its own fields (2026-11-15)

A UI kit's numeric stepping keeps a `NumberRange` that extends a
`StepRange`, and gives the range to its step functions,
`max_on_grid(&range.range)`; it makes a local copy whose step changes,
`NumberRange { range: StepRange { step, ..range.range }, ..range }`
(#1820). Both were refused: a flattened field lent whole was refused where
its type had `Clone`, `PartialEq`, `PartialOrd` or `Hash`, as those read
its object whole, the parent's keys too; one read by value, a read whole.

**A flattened struct lent whole or read whole by value is its own fields
wherever it's copied, cloned, compared, ordered or hashed** (one fact,
`Flattened::projected`, of the flatten owner; each consumer asks it):

- **The fact.** A struct a flattened field holds, of named fields of a
  crate's (`bindings::plain_struct`, its own flattened fields so too), that
  a body lends whole or reads by value anywhere it isn't read through
  (`bindings::read_through`). **A library takes every one in a struct a
  consumer can reach** (`library::reachable`): a consumer may lend it, and
  the library's own derived `==`, hash and clone of it, compiled once,
  must read its fields alone then too. That costs a library the spread
  form, `{ ...r }`, of such a struct's copies, where a program keeps it of
  each it never reads whole; no contract field is needed, as the library
  answers for its own code and a consumer for its own.
- **A copy or clone is its projection**, `{ min: range.min, max:
  range.max, step: range.step }`, never `{ ...range }` or the object:
  read by value, `range.range`, copied out of a reference, `*range`
  (`mir/operands.rs`), a derived clone (`std_impls/clones.rs`), built by
  the copy owner (`copies::projection`), each field copied in turn where
  it's changed in place, and a flattened field's keys of the same object.
  A local of one, never a parent, is shared as before.
- **A derived `==` compares field by field** (`custom_eq`), as the derive
  does, never `$eq` of every key; a derived order and hash do already.
- **A map's or a set's key found by value is its fields**, `$keySet((key)
  => ({ cols: key.cols, rows: key.rows }))` (`holds_identity`,
  `key_parts`), so a parent object lent as a key finds what its copy was
  kept under.
- **Still refused:** lent mutably, `&mut range.range`, or written whole,
  which may replace it; a name its parent has too (ADR 0205); and a keyed
  enum's, an untagged enum's or a state's whose clone or `==` reads its
  object whole.

Codex's local copy reads `{ step: 1, min: r.min, max: r.max, largeStep:
r.largeStep, smallStep: r.smallStep }`, its fields in the order Rust
evaluates them (ADR 0674).

And a crash it found (P1, #1838): a derived order, or a field-by-field
`==`, of a struct with a flattened field read the field as a nested
object, `a.range.min`, which threw. A flattened field's part is the
parent itself (`ordering.rs`, `std_impls.rs`).

Case ([0262](0262-when-rust-and-js-disagree.md)): none, as nothing reads
the parent's other keys, a compile error lifted.

Held by the corpus's `flattened_struct_lent_whole` (lent to a function,
copied, cloned, compared, ordered, a set's and a map's key, hashed, read
by value into a binding and a struct update, through two levels of
flattening, and a flattened struct never read whole, whose copy is
shared), against native Rust in both builds; the pair
`flattened_struct_lent_by_a_consumer`, a library's `==`, hash and clone
of a struct its consumer lends; `test/lowering.test.ts`'s "a flattened
struct copied out of what's lent is its own fields", JS given each copy,
and "a flattened field lent whole is refused where its type reads it
whole, or lent mutably", a keyed enum's clone refused; and
`test/jsx-props.test.ts`'s props read by value. Mutations
`projected-read-whole-unseen`, `projected-read-through-too` (a snapshot's),
`projected-library-unreached` (`bindings/flattened.ts`),
`projected-eq-whole`, `flattened-part-eq-nested` (`std_impls.ts`),
`projected-key-whole` (`std_types/map_keys.ts`), `projected-read-shared`,
`projected-deref-shared` (`mir/operands.ts`), `projected-clone-shared`
(`std_impls/clones.ts`), `projected-binding-refused` (`mir/assign.ts`),
`projected-lent-refused`, `projected-written-read`,
`projected-read-refused` (`analysis/validation.ts`) and
`flattened-part-cmp-nested` (`ordering.ts`); `flattened-read-through-deref`
retargeted to `bindings/flattened.rs`, where `read_through` is now.
