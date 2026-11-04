# 1077. A function JS can't call as declared isn't declared

Status: Accepted (#549, the user's decision, 2026-11-03, option B);
amended (#559): a namespace's function, an object's and a class's member
too; amended (#550): a crate's trait is declared as the type of its
dictionaries, and a function of one is declared with it. Amends
[0196](0196-typescript-declarations.md), whose `.d.ts` declared every
`pub` function, and the guide's "A type parameter with no bound takes
none" ([0145](0145-type-facts.md)'s facts take one).

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
  are evidence too. They're out of this decision: a JS caller that leaves
  `location` out gets a panic of no location, and `N` is a number a caller
  can give.

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
