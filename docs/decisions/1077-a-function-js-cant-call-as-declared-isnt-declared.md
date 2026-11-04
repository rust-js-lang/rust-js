# 1077. A function JS can't call as declared isn't declared

Status: Accepted (#549, the user's decision, 2026-11-03, option B);
amended (#559): a namespace's function, an object's and a class's member
too; amended (#550): a crate's trait is declared as the type of its
dictionaries, and a function of one is declared with it; amended (#560):
a caller's location and a const parameter's value are declared; amended
(#619, #620): a `&mut` that's a box is declared as the box, and a `dyn`
as its pair, `{ value: unknown; impl: Trait<unknown> }` (the user's
decision, 2026-11-04). Amends
[0196](0196-typescript-declarations.md), whose `.d.ts` declared every
`pub` function, and the guide's "A type parameter with no bound takes
none" ([0145](0145-type-facts.md)'s facts take one); amended by [1086](1086-a-mut-refs-js-form-is-mut-ref-forms.md): a declared `&mut`'s type is `MutRefForm::ts_type`'s, asked through `SignatureForms`; amended by [1091](1091-a-functions-declaration-prints-its-js-signature.md): what a function takes after its arguments, and which of it a JS caller can't give, is its `JsSignature`'s, lowering's `evidence_slots`' list.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change. A JS
caller that follows the declaration gets a value the Rust can't give, so
the declaration that promises it goes. The function's JS is unchanged:
nothing a Rust caller does differs.

## Context

A generic function's JS takes evidence after its arguments, in
`evidence_slots`' order:
- a bound's dictionary, `shown(value, TDisplay)` ([0049](0049-traits-and-generics.md));
- a fact of a type parameter, `size(TSize)`, `named(TName)` ([0145](0145-type-facts.md),
  since 7a172532e);
- a `T: 'static` made a `dyn Any` or compared by `TypeId::of`, its
  `TypeId`, `isU32(value, TId)` ([0331](0331-dyn-any.md), since
  0f6a2171c).

A Rust caller gives each. The `.d.ts` (since 7bab30260) declared each such
function by its Rust parameters alone, `export function size<T>(): number;`,
and a JS or TypeScript caller has no value to give. Found studying a generic
API for alien-signals' port (`signal<T>` over `Rc<dyn Any>`):

```ts
size()        // undefined, declared `number`
named()       // undefined, declared `string`
isU32(5)      // false
shown(5)      // TypeError: Cannot read properties of undefined (reading 'fmt')
signal(1)     // works by accident: every `TId` is `undefined`, so every downcast matches
```

[0909](0909-a-type-parameters-value-is-any-value.md) met it in React's
`push(cursor, value, fiber, TId)` and moved the port to `js::any_of`; the
compiler still declared what it couldn't keep.

## Decision

**A function whose JS takes, after its arguments, what a JS caller can't
give isn't declared in its `.d.ts`. A `//` note stands where its
declaration would be:**

```ts
// `size` isn't declared: its JS takes `T`'s size_of after its arguments, which a JS caller can't give (ADR 1077).
```

- **What a JS caller can't give** (`Declarations::ungiven`, in
  `declarations/functions.rs`, asking `evidence_slots`' own owners):
  - a fact of a type parameter, any of `TypeFact`'s: `size_of`,
    `align_of`, `type_name`, the `TypeId`, `needs_drop`, a map's class,
    an unstable sort, and the iterator facts. These are `type_facts`',
    which `facts_asked` reads.
  - a bound's dictionary, `bounds`'. A std trait's is the runtime's own,
    and a crate's own impl dictionary, `circleShape`, is exported in its
    JS, but no declaration declares one yet (#550), so TypeScript can't
    name it.
  What JS leaves out, a copy's (`T: Copy`), a clone of what's its own
  clone, a drop, isn't such a slot: those functions are declared as
  before.
- **A free function** is a note. (A default export can't be one: Rust
  names no generic function, nor one of an `impl Trait`, without its
  types.) **A method of a type's object**, `Counter.sizeOf`, is
  left out of the object; an object's member can't carry a note. **An `export { a
  as b }`** of one left out is left out.
- **Its JS is as before**, and a Rust caller calls it as before: another
  module's, a library's consumer's through its manifest. Nothing is
  refused.

The note's text names `TypeFact::described`, the one table of what each
fact is called. Lowering's "`size_of` of a type parameter" refusal reads
the same table.

## Why

- **A declaration says what a JS caller can call.** One that promises
  `size(): number` and gives `undefined` is a wrong answer at the
  boundary. TypeScript now refuses the import at its compile time,
  `Module has no exported member 'size'`.
- **Rust callers keep working.** `pub` isn't "JS calls it": index.rs calls
  system.rs's `pub fn`, and a library's consumer calls its generic
  functions with each fact its manifest names.

## Rejected

- **A: declare what the JS takes**, `size<T>(TSize: number): number`,
  `isU32<T>(value: T, TId: string)`. Truthful, but it publishes rustc's
  type names and layout as an API no JS caller can meet.
- **C: refuse a type fact only where JS alone reaches the function**: the
  default export, the package's entry module, `js::export_default!`. An
  arbitrary line, and JS still imports any module's file.
- **D: refuse, at compile time, a declared function that takes a fact.**
  It turns valid Rust only Rust calls into a refusal: a library's generic
  function its consumers call, a module's `pub fn` another module calls.

## Consequences

- Untyped JS can still import such a function and call it without what it
  takes, and gets a wrong answer. The guide says so, and what to write for
  a function JS calls: one of a concrete type, `js::any_of`/`js::cast` for
  a value of any type ([0909](0909-a-type-parameters-value-is-any-value.md)),
  or no `pub` for one only Rust calls.
- A function of a bound comes back into the `.d.ts` where its trait's
  dictionaries are declared (#550, amended below).
- A `#[track_caller]` function's `location` and a const parameter's `N`
  are evidence too, declared since #560 (amended below).

Held by `test/declarations-ungiven.test.ts`, which has a note for each
fact kind and a dictionary, a type's method left out of its object, an
export by another name left out, a `T: Clone`
still declared, and `tsc` refusing the import, and by the corpus's
`pub_type_facts`, which calls each from Rust against native Rust.
Mutations in `lower/declarations/functions.ts`, `lower/declarations.ts`
and `lower/declarations/namespaces.ts`.

## Amended (#559): every member

1639d621b left three places declaring such a function by its Rust
parameters alone, the same wrong answer: a module namespace's function
(ADR 1037), `sizeOf(TSize)` declared `sizeOf<T>(): number`; an object of
methods' method (ADR 0943); and a class's method or static (ADR 0761),
`Widget.prototype.isU32(TId)`, `Widget.shown(value, TDisplay)`. Each is
left out now, as a type's object method is, and every note is
`Declarations::leave_out`'s: it's kept in `notes` and printed after the
declaration it's of, the module's next item first taking them, `//
\`Widget.isU32\` isn't declared: ..`. A class's trait method is never
one: a class implements no trait whose methods have type parameters (ADR
0761 refuses it). Held by `test/declarations-ungiven.test.ts`'s
"a namespace's, an object's and a class's member whose JS takes a fact is a
note" (its program `test/fixtures/ungiven_members.rs`), and the mutations
`ungiven-namespace-function-declared`, `ungiven-object-method-declared`,
`ungiven-class-member-declared` and `ungiven-notes-at-the-end`.

## Amended (#550): a trait is the type of its dictionaries

Case ([0262](0262-when-rust-and-js-disagree.md)): none. Nothing a program
does changes: its JS is as before, and its `.d.ts` declares what that JS
is, so a TypeScript caller can give what a Rust caller gives.

The JS exports each impl's dictionary of a `pub` trait, `export const
circleShape = { area, describe }` (ADRs 0049, 0592), and a generic
function takes it after its arguments, `totalArea(shapes, SShape)`. The
`.d.ts` declared neither, so every function of a bound was a note.

**A `pub` trait of the crate's whose dictionaries hold its methods alone
is declared as their type, each dictionary its module exports as a `const`
of it, and a bound's dictionary of it as a parameter:**

```ts
export interface Shape<Self> {
    area(self: Self): number;
    describe(self: Self): string;
}
export const circleShape: Shape<Circle>;
export function totalArea<S>(shapes: S[], SShape: Shape<S>): number;
```

- **What a dictionary holds is `traits::method_dictionary`'s answer**,
  beside `held_supertraits` and `item_bounds`, which say what else
  `lower_dictionary` puts in one: each method, by `fn_name`, its value
  first, as the trait's signature says, `Self` the interface's parameter;
  a `$drop`, read `?.` where a `dyn` is dropped, needs no declaring. It's
  `None`, and the trait isn't declared, where a dictionary holds or a
  method takes what its signature doesn't say: a supertrait's dictionary;
  an associated type, its bounds' dictionaries, an `async fn`'s future;
  a constant, which is in it only where generic code reads it (ADR 0100);
  a method's own type parameters' evidence, or its caller's location (ADR
  0427, its declaration's `#[track_caller]` or an impl's); a `&mut`
  anywhere in a method's signature, which may be a box or a handle (ADR
  0099); a `where` of a method's own, which an impl's type may not meet,
  so its dictionary leaves the method out. Nor is a trait of type
  parameters of its own, each impl's dictionary for its arguments, a
  marker, which has none, a trait of classes, whose methods are its
  classes' prototypes' and which has none either (rd283), or a library's.
- **An impl's dictionary is declared where it takes nothing**, an impl of
  no type or const parameters, its module's `const` (ADR 0592), by its
  name (`FnInfo`'s); the trait is `pub`, so its module exports it. A
  generic impl's, `wrapperShape(TDisplay)`, a function of its own bounds'
  dictionaries, isn't declared yet, nor is a std trait's: `Display`'s is
  the runtime's own.
- **A function's evidence is `Declarations::evidence`'s**, in
  `evidence_slots`' order: each bound's dictionary of a declared trait,
  `Shape<S>`, imported from the trait's module, named as its JS names it,
  `bound_word`'s, the one word of both; or, where one isn't declared or a
  fact is asked, the first a JS caller can't give, which `leave_out`
  notes as before. A trait's method takes none of its own trait's: its
  dictionary's type declares it. After a dictionary, a trailing `Option`
  isn't optional, `prefix: string | null | undefined`: a caller gives it
  so its dictionary is in its place. (A rest parameter takes no evidence:
  lowering refuses one, ADR 0664.)
- **A dictionary TypeScript writes is one generic code calls**, `{ area:
  (s) => s.side ** 2, describe: (s) => "a square" }`, as it calls an
  impl's: each entry is called as the trait's method is, of what its
  signature says.

Rejected: **declaring each trait with what its dictionaries may hold
besides**, a supertrait's as a getter of its own interface, a constant as
a property: a constant is in a dictionary only where generic code reads
it, so the interface would say what one may not have, and a std
supertrait's dictionary has no declaration. These traits stay notes until
a need shows; each is a rule of `method_dictionary`'s to extend.

Held by `test/declarations-ungiven.test.ts`'s "a trait is the type of its
dictionaries, which a function's declaration takes" (its program
`test/fixtures/declared_dictionaries.rs`): the interfaces, the impls'
`const`s, another module's trait imported, a `None` before a dictionary,
two bounds, and a note for each trait that isn't declared, a generic
impl's dictionary undeclared, and its "a namespace's, an object's and a
class's member" a trait of classes, none of its dictionary; `tsc` checks a TypeScript caller giving
the crate's dictionaries and one of its own, refuses a dictionary of
another type, none and the imports of what isn't declared, and the
caller's run gives what Rust's calls give. Mutations in
`lower/traits.ts`, `lower/declarations/functions.ts`,
`lower/declarations/dictionaries.ts` and `lower/declarations.ts`.

## Amended (#560): a caller's location and a const parameter's value

Case ([0262](0262-when-rust-and-js-disagree.md)): results change. Each
was declared as if it weren't there, `checked(n: number): number`,
`zeros(): number[]`, and a JS caller that followed the declaration got
what Rust can't give: `checked(0)` threw `TypeError: Cannot read
properties of undefined (reading 'file')` where Rust panics `zero at
src/lib.rs:12:5`; `here()` was `undefined`; `zeros()` was `new
Array(undefined).fill(0)`, `[0]`, where `zeros::<3>()` is `[0, 0, 0]`;
`big()`, a `const N: u64`, `undefined`, declared `bigint`. The JS is
unchanged; the declaration now says what it takes.

**Both are what a JS caller can give, so each is declared, required, in
`evidence_slots`' order, before the dictionaries:**

```ts
export function checked(n: number, location: {
    readonly file: string;
    readonly line: number;
    readonly column: number;
}): number;
export function zeros(N: number): number[];
export function big(N: bigint): bigint;
```

- **A caller's location** is `{ file, line, column }` (ADR 0427,
  `location::PARTS`, the one list of what's made and what's declared), what
  a Rust caller gives of where it calls from. A JS caller gives its own,
  and the function reads it as Rust's reads its caller's: `checked(0, {
  file: "app.ts", line: 3, column: 7 })` panics `zero at app.ts:3:7`.
  Which functions take one is `location::takes_location`'s, the crate's
  `caller_locations` or a library's manifest, the question lowering asks
  too. A `Location` is declared so wherever it's named, `here()`'s
  `&'static Location` too, which was `any`.
- **A const parameter's value** is the number, `bool` or `char` it is, of
  its type as declared, `usize` a `number` and `u64` a `bigint`, named as
  the Rust names it, `N`. `zeros(3)` is `[0, 0, 0]`, as `zeros::<3>()`.
- **Neither is optional**, and after either a trailing `Option` isn't
  either, as after a dictionary.

Rejected:
- **A note, as a fact's.** A fact is rustc's knowledge of a type no JS
  caller has (above); a location and a const's value are values a caller
  chooses, as a Rust caller chooses `zeros::<3>`.
- **`location?`, optional.** Left out, it's `undefined`, and the body
  reads its `file`: a `TypeError` where Rust names a place.
- **A default, `location = { file: "src/lib.rs", .. }` of the function's
  definition**, as a dictionary's entry and a function pointer's shim
  have it (ADR 0427). It changes the JS of every exported tracked
  function, and a JS caller isn't a shim: it can say where it calls from.

Held by `test/declarations-ungiven.test.ts`'s "a function's caller's
location and a const parameter's value are declared" (its program
`test/fixtures/evidence_given.rs`): each declared, a type's method of its
impl's const parameter, one of both a location, a const and a
dictionary in their order; `tsc` checks a TypeScript caller and refuses
one that leaves either out; the caller's run gives what Rust's calls
give, a panic naming the location given. Mutations in
`lower/declarations/functions.ts` and `lower/declarations.ts`.

## Amended (#619, #620): a `&mut` that's a box, and a `dyn`

Case ([0262](0262-when-rust-and-js-disagree.md)): results change. The JS
is unchanged; its declaration said what it isn't, and a JS caller that
followed it got what Rust can't give.

- **A `&mut` declared as its value** (rd760, #619): `bump(n: &mut u32)`
  was `bump(n: number)` and `set<T>(slot: &mut T, v: T)` `slot: T`, where
  the JS reads and writes `n.value` (ADRs 0074, 0099). `bump(1)`,
  `set(1, 2)` and `shout("x")` of a `&mut String` threw `TypeError:
  Attempted to assign to readonly property` where Rust changes the place;
  `largest(items): number` gave a handle, and `largest([1, 2]) + 1` was
  `"[object Object]1"`.
- **A `dyn` of a trait not a `Fn`'s declared a function** (rd761, #620):
  `square(side): Box<dyn Shape>` was `(...args: any[]) => any`, where the JS
  gives `{ value: { side }, impl: squareShape }`; `square(1)()` threw
  `square(1) is not a function`.

**A `&mut` its JS has as a cell, a box or a handle, is declared as that,
`{ value: T }`; one to an object is the object. A `dyn` of a trait whose
dictionaries are declared is its pair, `{ value: unknown; impl:
Shape<unknown> }`; of any other trait not a `Fn`'s, `any`:**

```ts
export function bump(n: {
    value: number;
}): void;
export function set<T>(slot: {
    value: T;
}, value: T): void;
export function tick(counter: Counter): void;
export function square(side: number): {
    value: unknown;
    impl: Shape<unknown>;
};
export function stepper(): any;
```

- **Which `&mut` is a cell is lowering's answer, asked**, not made again:
  `is_cell_pointee` (`representation.rs`), of which `param_is_box`, what a
  body's parameter is, is the same rule of the function's own bounds;
  `dynamic_trait` (`traits/dyns.rs`) says which `dyn` is a pair, and of
  what trait. The pipeline gives the declarations a `Representation`
  that asks each in a context of the item whose signature is declared,
  `contexts::fn_cx`'s, as its body is lowered in. A method's own type
  parameters are the method's, so `function` asks of the function
  itself: a method's `&mut F` of an `F: FnMut` is the function, a box
  where it's asked of its type. A `&mut` to a value JS can't change in
  place, a number, a `String`, an `Option`, an enum without fields, or a
  counted `Rc`, and to a type parameter that's no `Fn` or iterator, is a
  cell, as it's a parameter, a result, a closure's parameter, `(value: {
  value: number }) => void`, or a method's `self`, `raise(level: { value:
  Level })`; to a struct of fields, a `Vec`, a slice, a closure or a
  `dyn` pair, it's the object as before.
- **A pair is of a value of some type**, which no type parameter of the
  function's names: each `Box<dyn Shape>` of a `Vec` may be another's. Its
  value is `unknown` and its dictionary `Shape<unknown>`, as TypeScript
  writes a value of some type, `Promise<unknown>`, `ReadonlyArray<unknown>`;
  a method's parameter is bivariant, so `{ value: { side: 2 }, impl:
  squareShape }` is one. The trait is imported from its module, as a
  bound's dictionary is (#550). The user's decision (2026-11-04), knowing
  it also admits a pair of a value and another type's dictionary, `{ value:
  5, impl: squareShape }`, which TypeScript can't tell apart without a
  type parameter the `Vec` and the result can't have (Rejected, below).
- **A `dyn` of another trait is `any`**, as what has no type here is (ADR
  0196): its dictionary isn't declared, a std trait's or one whose
  dictionaries hold what else (#550), a JS iterator's (`dyn Iterator`).
  `any` says nothing of it, where the function said what it isn't.
- **`drops_observed.keep` gives the drop parameters it keeps back**, of
  what the bodies were lowered with, which is as it was: the declarations,
  made after, ask a context of the crate's facts, which hold those.

Rejected:
- **A note, leaving each such function out.** A JS caller can give a box
  and a pair, and TypeScript can say what each is.
- **`slot: T | { value: T }`, or a pair of a type parameter of the
  function's, `areaOf<S>(shape: { value: S; impl: Shape<S> })`.** The
  first says a value is given where the JS reads its `value`; the second
  holds every `dyn` of a function to one type, where a `Vec<Box<dyn
  Shape>>`'s are each of their own, and can't say what a function gives.
- **Reading which parameters are boxes back from the lowered JS.** It's
  lowering's decision, asked of its owner; the JS's shape is no API.

Not yet: a type parameter's `Fn` bound is declared as no bound, `call<F>(f:
F)`, nor are a trait's `&mut self` methods' dictionaries (#550), which
take a box whatever the impl's type. (Both are since
[1091](1091-a-functions-declaration-prints-its-js-signature.md), amended:
#661, #662.)

Held by `test/declarations-ungiven.test.ts`'s "a `&mut` that's a box is
declared as its box, and a `dyn` as its pair" (its program
`test/fixtures/declared_boxes.rs`): each `&mut` of a number, a type
parameter, a `String`, an `Option`, a result's, a closure's parameter's
and a fieldless enum's `self` a box, of a struct, a `Vec` and a closure
the object, a method's `F: FnMut` its own, a `Box<dyn Shape>`, a `&dyn
Shape` and a `Vec` of them a pair with `Shape` imported, a `dyn` of a
trait not declared `any`; `tsc` checks a TypeScript caller of each and
refuses each call the declarations before allowed, `bump(1)`, `set(1, 2)`,
`shout("x")`, `Level.raise("Low")`, `largest([1]) + 1`, `square(1)()`,
`areaOf(() => 1)`, and a box given where an object is, `tick({ value: ..
})`; the caller's run gives what the same calls give natively. Mutations
in `lower/declarations.ts`, `lower/declarations/dictionaries.ts` and
`lower/declarations/functions.ts`.
